use crate::DiffOperation;
use crate::context::{AnyTradeoff, Impl};
use crate::networked_types::primitive::{PrimitiveSerDes, SliceSerDes, usize32};
use crate::tick::TickType;
use std::marker::PhantomData;
use std::mem;
use std::rc::Rc;

#[cfg(feature = "server")]
use {crate::NetVisibility, crate::tick::TickID, std::collections::HashMap};

//diff serializer's purpose is to capture any and
//all changes to state. a serializer of differences.
//the generic param functionally does nothing, and
//should not cause any function duplication bloat in
//an optimized build
#[derive(Default)]
pub struct DiffSerializer<Tradeoff: AnyTradeoff> {
	//write the PREVIOUS value of a state in order to
	//undo+resimulate it later. will be read back to
	//front (i=len to i=0). contains multiple ticks
	//worth of state changes (however many are still
	//considered unfinalized predictions). remember
	//rollback is a LOCAL backup consisting only of
	//fields that exist locally and will never be sent
	//over wire
	pub(crate) rollback_buffer: Vec<u8>, //raw packed bytes representing diffs to the state
	rollback_enabled: bool,              //rollback is disabled during unrollbackable events
	rollback_prv_path: Option<Rc<Vec<usize32>>>,

	//write the CURRENT value of a state to transmit
	//over the wire. will be read front to back (i=0
	//to i=len). contains only one tick at a time
	//before sending+resetting
	#[cfg(feature = "server")]
	tx: HashMap<usize32, TxData>,
	#[cfg(feature = "client")]
	tx: TxData,

	//think of it like rollback = instructions for
	//undoing changes to simulation state, and
	//tx = instructions for redoing/replicating/
	//replaying changes to simulation state on a
	//different device on the network
	phantom_menace: PhantomData<Tradeoff>,
}

struct TxData {
	buffer: Vec<u8>,

	#[cfg(feature = "server")]
	enabled: bool,
	#[cfg(feature = "server")]
	cur_path: Rc<Vec<usize32>>,
}

impl Default for TxData {
	fn default() -> Self {
		Self {
			buffer: Vec::default(),

			#[cfg(feature = "server")]
			enabled: true, //<--important!
			#[cfg(feature = "server")]
			cur_path: Rc::default(),
		}
	}
}

impl DiffSerializer<Impl> {
	//---state change tracking---//
	//ser_rollback_begin and the server side version of ser_tx_begin
	//should be called in pairs, one after the other. caller must
	//serialize the diff operation

	pub fn ser_rollback_begin(&mut self, path: &Rc<Vec<usize32>>) -> Option<&mut Vec<u8>> {
		if self.rollback_enabled {
			self.ser_rollback_navigate_to(path);
			Some(&mut self.rollback_buffer)
		} else {
			None
		}
	}

	//only the server sends authoritative state updates
	#[cfg(feature = "server")]
	pub fn ser_tx_begin(
		&mut self,
		path: &Rc<Vec<usize32>>,
		visibility: NetVisibility,
	) -> impl Iterator<Item = &mut Vec<u8>> {
		self.tx
			.iter_mut()
			.filter(move |(tx_client_id, tx)| {
				//scope filtering: skip sending this state to any
				//client who doesn't need to know about this change
				tx.enabled
					&& match visibility {
						NetVisibility::Public => true,
						NetVisibility::Private => false,
						NetVisibility::Owner => {
							let modified_client_id = path[1];
							**tx_client_id == modified_client_id //yuck
						}
					}
			})
			.map(|(_, tx)| {
				Self::ser_tx_navigate_to(tx, path);
				&mut tx.buffer
			})
	}

	//only the client sends authoritative input updates
	#[cfg(feature = "client")]
	pub fn ser_tx_begin(&mut self) -> &mut Vec<u8> {
		&mut self.tx.buffer
	}

	//---tick lifecycle---//

	#[cfg(feature = "server")]
	pub fn tx_toggle_client(&mut self, id: usize32, enable: bool) {
		self.tx.get_mut(&id).unwrap().enabled = enable;
	}

	pub fn ser_begin_tick(&mut self, tick_type: TickType, #[cfg(feature = "server")] tick_id: TickID) {
		self.rollback_enabled = tick_type == TickType::Predicted;
		self.rollback_prv_path = None;

		if self.rollback_enabled {
			DiffOperation::RollbackTickSeparator.ser_rollback(&mut self.rollback_buffer);
		}

		#[cfg(feature = "server")]
		for client in self.tx.values_mut() {
			Self::ser_tx_begin_tick(client, tick_type, tick_id);
		}
	}

	#[cfg(feature = "server")]
	fn ser_tx_begin_tick(client: &mut TxData, tick_type: TickType, tick_id: TickID) {
		if client.enabled {
			let buffer = &mut client.buffer;
			tick_id.ser_tx(buffer);
			tick_type.ser_tx(buffer);
		}
	}

	//get the finalized data to send over the wire.
	//called per-webtransport connection
	pub fn tx_end_tick(&mut self, #[cfg(feature = "server")] client_id: usize32) -> Option<Vec<u8>> {
		//server is sending simulation state
		#[cfg(feature = "server")]
		let (tx, enabled) = {
			let tx = self.tx.get_mut(&client_id).unwrap();
			let enabled = tx.enabled;
			(tx, enabled)
		};

		//client is sending input state
		#[cfg(feature = "client")]
		let (tx, enabled) = (&mut self.tx, true);

		if enabled {
			//server buffer is guaranteed to always start
			//with a tick id. empty client buffer is valid,
			//and the server still needs to know that
			//nothing has changed since the last tick
			#[cfg(feature = "server")]
			{
				debug_assert!(tx.buffer.len() > 0);

				tx.cur_path = Rc::default();
			}

			let ret = mem::take(&mut tx.buffer);
			Some(ret)
		} else {
			debug_assert_eq!(tx.buffer.len(), 0);
			None
		}
	}

	pub fn ser_rollback_end_tick(&mut self) {
		if self.rollback_enabled {
			self.ser_rollback_navigate_to(&Rc::default());
		}
	}

	#[cfg(feature = "server")]
	pub fn on_connect(&mut self, client_id: usize32, tick_id: TickID) {
		self.tx.insert(client_id, TxData::default()); //default = tx enabled
		Self::ser_tx_begin_tick(self.tx.get_mut(&client_id).unwrap(), TickType::NetEvents, tick_id);
	}

	#[cfg(feature = "server")]
	pub fn on_disconnect(&mut self, client_id: usize32) {
		self.tx.remove(&client_id).unwrap();
	}

	//---diff path navigation---//

	//to avoid having to write the full path on every single
	//serialize operation, only write when the path changes
	//relative to the current path.
	//path sizes are stored as u8. this forces a requirement that
	//there cannot be more than 256 layers of nested collection
	//types. deal with it
	//[field id, element id, field id, element id...]
	fn ser_rollback_navigate_to(&mut self, new_path: &Rc<Vec<usize32>>) {
		//the thing that makes rollback navigation
		//clunkier than tx navigation is that the
		//path must be written to the buffer AFTER
		//fields at this path have been modified.
		//(remember parsing rollback happens in
		//reverse order.)
		//if prv_path is none, the tick is brand new.
		//do not write to the rollback buffer.
		if let Some(prv_path) = &self.rollback_prv_path {
			if let Some(shared_len) = find_first_mismatch(&new_path, &prv_path) {
				let new_len = new_path.len();
				let prv_len = prv_path.len();
				let buffer = &mut self.rollback_buffer;

				if prv_len > shared_len {
					prv_path[shared_len as usize..prv_len as usize].ser_rollback(buffer);
					(((prv_len - shared_len) / 2) as u8).ser_rollback(buffer);
					DiffOperation::NavigateDown.ser_rollback(buffer);
				}

				if new_len > shared_len {
					if shared_len == 0 {
						DiffOperation::NavigateReset.ser_rollback(buffer);
					} else {
						(((new_len - shared_len) / 2) as u8).ser_rollback(buffer);
						DiffOperation::NavigateUp.ser_rollback(buffer);
					}
				}

				self.rollback_prv_path = Some(new_path.clone());
			}
		} else {
			self.rollback_prv_path = Some(new_path.clone());
		}
	}

	#[cfg(feature = "server")]
	fn ser_tx_navigate_to(tx_data: &mut TxData, new_path: &Rc<Vec<usize32>>) {
		if let Some(shared_len) = find_first_mismatch(&tx_data.cur_path, &new_path) {
			let cur_len = tx_data.cur_path.len();
			let new_len = new_path.len();
			let buffer = &mut tx_data.buffer;

			if cur_len > shared_len {
				if shared_len == 0 {
					DiffOperation::NavigateReset.ser_tx(buffer);
				} else {
					DiffOperation::NavigateUp.ser_tx(buffer);
					(((cur_len - shared_len) / 2) as u8).ser_rollback(buffer);
				}
			}

			if new_len > shared_len {
				DiffOperation::NavigateDown.ser_tx(buffer);
				(((new_len - shared_len) / 2) as u8).ser_tx(buffer);
				new_path[shared_len as usize..new_len as usize].ser_tx(buffer);
			}

			tx_data.cur_path = new_path.clone();
		}
	}
}

fn find_first_mismatch<T: PartialEq>(vec1: &[T], vec2: &[T]) -> Option<usize> {
	//compare 2 elements at a time (one nav
	//level = 2 elements)
	for (i, (level1, level2)) in vec1.chunks(2).zip(vec2.chunks(2)).enumerate() {
		if level1[0] != level2[0] || level1[1] != level2[1] {
			return Some(i * 2);
		}
	}

	if vec1.len() != vec2.len() {
		return Some(vec1.len().min(vec2.len()));
	}

	None
}
