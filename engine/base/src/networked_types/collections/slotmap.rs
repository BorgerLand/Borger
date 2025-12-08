use crate::constructors::ConstructCollectionOrUtilityType;
use crate::context::AnyTradeoff;
use crate::diff_des::DiffDeserializeState;
use crate::diff_ser::DiffSerializer;
use crate::networked_types::primitive::{PrimitiveSerDes, SliceSerDes, usize_to_32, usize32};
use crate::presentation_state::CloneToPresentationState;
use crate::snapshot_serdes::SnapshotState;
use crate::untracked::UntrackedState;
use crate::{ClientStateKind, DeserializeOopsy, DiffOperation, NetState};
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

#[cfg(feature = "client")]
use {
	crate::context::Impl, crate::simulation_state::ClientState, std::any::TypeId, std::collections::VecDeque,
};

//like a hashmap, except the key is an internally generated, unique, numeric id.
//currently does not do any sort of generational tracking
//guarantees:
//- values are stored in contiguous memory for fast iteration
//- get+remove (random accesses) are O(1)
//- add is amortized O(1), but O(n) if internal data structures need to resize
//- iteration order is deterministic but not in order of insertion*
//- 	*it will only be in order of insertion so long as elements are not removed from the "middle"
//- 	*you can grab the most recently inserted slot with .iter().last(), if no slots have been removed since inserting it

//---simulation_state---//

#[derive(Debug)]
pub struct SlotMap<V: NetState> {
	_diff_path: Rc<Vec<usize32>>,
	field_id: usize32,

	#[cfg(feature = "server")]
	visibility: NetVisibility,

	slots: Vec<(usize32, V)>,
	random_access: HashMap<usize32, usize32>, //<slot id, physical index>

	next_id: usize,
	reclaimed_ids: Vec<usize32>,
}

//---presentation_state---//

impl<V: NetState> CloneToPresentationState for SlotMap<V> {
	type PresentationState = Vec<(usize32, V::PresentationState)>;

	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState {
		let mut clone = Vec::with_capacity(self.slots.len());
		for slot in self.slots.iter() {
			clone.push((slot.0, slot.1.clone_to_presentation()));
		}

		clone
	}
}

//---constructors---//

impl<V: NetState> ConstructCollectionOrUtilityType for SlotMap<V> {
	fn construct(
		path: &Rc<Vec<usize32>>,
		field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self {
		Self {
			_diff_path: path.clone(),
			field_id,

			#[cfg(feature = "server")]
			visibility,

			slots: Vec::new(),
			random_access: HashMap::new(),

			next_id: 0,
			reclaimed_ids: Vec::new(),
		}
	}
}

//---diff_ser---//

impl<V: NetState> SlotMap<V> {
	pub fn add(&mut self, diff: &mut DiffSerializer<impl AnyTradeoff>) -> (usize32, &mut V) {
		self.add_with_client_owned(ClientStateKind::NA, diff)
	}

	pub(crate) fn add_with_client_owned(
		&mut self,
		client_kind: ClientStateKind,
		diff: &mut DiffSerializer<impl AnyTradeoff>,
	) -> (usize32, &mut V) {
		let id = self.use_next_id();
		let physical_index = self.len();

		self.slots
			.push((id, V::construct(&self.build_slot_path(id), client_kind)));
		self.random_access.insert(id, physical_index);

		let op = DiffOperation::TrackSlotMapAdd;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self._diff_path) {
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self._diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
		}

		let slot = self.slots.last_mut().unwrap();
		(slot.0, &mut slot.1)
	}

	//can't return the removed value! it would provide
	//a loophole for constructing new state objects.
	//the simulation state must own all instances.
	//unwrap to assert success
	pub fn remove(&mut self, id: usize32, diff: &mut DiffSerializer<impl AnyTradeoff>) -> Option<()> {
		let physical_index_u32 = self.random_access.remove(&id)?;
		let physical_index_usize = physical_index_u32 as usize;
		let removed_slot = self.slots.swap_remove(physical_index_usize);

		if physical_index_u32 < self.len() {
			//swap_remove moved the last slot to the emptied slot.
			//need to update its random_access
			*self
				.random_access
				.get_mut(&self.slots[physical_index_usize].0)
				.unwrap() = physical_index_u32;
		}

		self.reclaim_id(id);

		let op = DiffOperation::TrackSlotMapRemove;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self._diff_path) {
			removed_slot.1.ser_rollback_predict_remove(buffer);
			physical_index_u32.ser_rollback(buffer);
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self._diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
			id.ser_tx(buffer);
		}

		Some(())
	}

	//significantly more efficient in terms of bandwidth usage
	//compared to iter+remove
	pub fn clear(&mut self, diff: &mut DiffSerializer<impl AnyTradeoff>) -> usize32 {
		let len = self.len();
		if len == 0 {
			return 0;
		}

		let op = DiffOperation::TrackSlotMapClear;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self._diff_path) {
			self.ser_rollback_predict_remove(buffer);
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self._diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
		}

		self.slots.clear();
		self.random_access.clear();
		self.next_id = 0;
		self.reclaimed_ids.clear();

		len
	}

	pub fn get(&self, id: usize32) -> Option<&V> {
		if let Some(&physical_index) = self.random_access.get(&id) {
			Some(&self.slots.get(physical_index as usize).unwrap().1)
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, id: usize32) -> Option<&mut V> {
		if let Some(&physical_index) = self.random_access.get(&id) {
			Some(&mut self.slots.get_mut(physical_index as usize).unwrap().1)
		} else {
			None
		}
	}

	pub fn len(&self) -> usize32 {
		self.slots.len() as usize32
	}

	pub fn iter(&self) -> impl ExactSizeIterator<Item = (usize32, &V)> {
		self.slots.iter().map(|slot| (slot.0, &slot.1))
	}

	pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = (usize32, &mut V)> {
		self.slots.iter_mut().map(|slot| (slot.0, &mut slot.1))
	}

	pub fn ids(&self) -> impl ExactSizeIterator<Item = usize32> {
		self.slots.iter().map(|slot| slot.0)
	}

	pub fn values(&self) -> impl ExactSizeIterator<Item = &V> {
		self.slots.iter().map(|slot| &slot.1)
	}

	pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut V> {
		self.slots.iter_mut().map(|slot| &mut slot.1)
	}

	#[cfg(feature = "client")]
	pub(crate) fn random_access(&self, id: usize32) -> Option<usize> {
		self.random_access
			.get(&id)
			.map(|physical_index| *physical_index as usize)
	}
}

//---diff_des---//

//wrapper to eliminate slotmap's generics and allow
//DeserializeState trait to be dyn compatible
pub(crate) trait SlotMapDynCompat {
	fn get_des(&mut self, id: usize32) -> Option<&mut dyn DiffDeserializeState>;

	fn rollback_add(&mut self);
	fn rollback_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;
	fn rollback_clear(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;

	#[cfg(feature = "client")]
	fn rx_add(&mut self, diff: &mut DiffSerializer<Impl>);
	#[cfg(feature = "client")]
	fn rx_remove(
		&mut self,
		buffer: &mut VecDeque<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy>;
	#[cfg(feature = "client")]
	fn rx_clear(&mut self, diff: &mut DiffSerializer<Impl>);
}

impl<V: NetState> SlotMapDynCompat for SlotMap<V> {
	fn get_des(&mut self, id: usize32) -> Option<&mut dyn DiffDeserializeState> {
		self.get_mut(id).map(|v| v as &mut dyn DiffDeserializeState)
	}

	fn rollback_add(&mut self) {
		let id = self.slots.pop().unwrap().0;
		self.random_access.remove(&id).unwrap();
		self.reclaim_id(id);
	}

	fn rollback_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		let physical_index = usize32::des_rollback(buffer)?;
		let id = self.use_next_id();

		//since client deletion is unpredictable and will
		//never roll back, client_kind can be anything
		//here. all other constructors ignore the arg
		let mut value = V::construct(&self.build_slot_path(id), ClientStateKind::NA);
		value.des_rollback_predict_remove(buffer)?;

		let reincarnated_slot = (id, value);
		if physical_index == self.len() {
			//insert as last slot
			self.slots.push(reincarnated_slot);
		} else {
			//insert in the middle + move the current resident
			//of this slot back to the end
			let end_slot = mem::replace(&mut self.slots[physical_index as usize], reincarnated_slot);
			*self.random_access.get_mut(&end_slot.0).unwrap() = self.len();
			self.slots.push(end_slot);
		}

		self.random_access.insert(id, physical_index);

		Ok(())
	}

	fn rollback_clear(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		self.des_rollback_predict_remove(buffer)
	}

	#[cfg(feature = "client")]
	fn rx_add(&mut self, diff: &mut DiffSerializer<Impl>) {
		self.add(diff);
	}

	#[cfg(feature = "client")]
	fn rx_remove(
		&mut self,
		buffer: &mut VecDeque<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy> {
		let id = usize32::des_rx(buffer)?;
		self.remove(id, diff).ok_or(DeserializeOopsy::PathNotFound)
	}

	#[cfg(feature = "client")]
	fn rx_clear(&mut self, diff: &mut DiffSerializer<Impl>) {
		self.clear(diff);
	}
}

//---snapshot---//

impl<V: NetState> SnapshotState for SlotMap<V> {
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, client_id: usize32, buffer: &mut Vec<u8>) {
		(self.reclaimed_ids.len() as usize32).ser_tx(buffer);
		self.reclaimed_ids.ser_tx(buffer);
		(self.next_id as usize32).ser_tx(buffer);
		self.len().ser_tx(buffer);

		for slot in self.slots.iter() {
			slot.0.ser_tx(buffer);
			slot.1.ser_tx_new_client(client_id, buffer);
		}
	}

	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		client_id: usize32,
		buffer: &mut VecDeque<u8>,
	) -> Result<(), DeserializeOopsy> {
		let reclaimed_ids_len = usize32::des_rx(buffer)?;
		self.reclaimed_ids = <[usize32]>::des_rx(reclaimed_ids_len, buffer)?;
		self.next_id = usize32::des_rx(buffer)? as usize;
		let slots_len = usize32::des_rx(buffer)?;

		for physical_index in 0..slots_len {
			let id = usize32::des_rx(buffer)?;

			//nasty specialization hack: this is temporary,
			//with the thinking that the custom net visibility
			//scopes system will supercede owned/remote system
			let client_kind = if id == client_id && TypeId::of::<V>() == TypeId::of::<ClientState>() {
				ClientStateKind::Owned
			} else {
				ClientStateKind::Remote
			};

			let mut slot = V::construct(&self.build_slot_path(id), client_kind);
			slot.des_rx_new_client(client_id, buffer)?;

			self.slots.push((id, slot));
			self.random_access.insert(id, physical_index);
		}

		Ok(())
	}

	//this is expensive because it serializes every single
	//slot. if called, you've either manually called .clear(),
	//or a parent/outer collection called remove() and was
	//forced to serialize all inner collections. there is
	//potentially a large amount of data being rapidly rolled
	//back
	fn ser_rollback_predict_remove(&self, buffer: &mut Vec<u8>) {
		for slot in self.slots.iter().rev() {
			slot.1.ser_rollback_predict_remove(buffer);
			slot.0.ser_rollback(buffer);
		}

		self.len().ser_rollback(buffer);
		(self.next_id as usize32).ser_rollback(buffer);
		self.reclaimed_ids.ser_rollback(buffer);
		(self.reclaimed_ids.len() as usize32).ser_rollback(buffer);
	}

	fn des_rollback_predict_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		let reclaimed_ids_len = usize32::des_rollback(buffer)?;
		self.reclaimed_ids = <[usize32]>::des_rollback(reclaimed_ids_len, buffer)?;
		self.next_id = usize32::des_rollback(buffer)? as usize;
		let slots_len = usize32::des_rollback(buffer)?;

		for physical_index in 0..slots_len {
			let id = usize32::des_rollback(buffer)?;
			let mut slot = V::construct(&self.build_slot_path(id), ClientStateKind::NA);
			slot.des_rollback_predict_remove(buffer)?;

			self.slots.push((id, slot));
			self.random_access.insert(id, physical_index);
		}

		Ok(())
	}
}

//---untracked---//
impl<V: NetState> UntrackedState for SlotMap<V> {
	fn reset_untracked(&mut self) {
		for slot in self.values_mut() {
			slot.reset_untracked();
		}
	}
}

//---misc---//

impl<V: NetState> SlotMap<V> {
	fn use_next_id(&mut self) -> usize32 {
		self.reclaimed_ids.pop().unwrap_or_else(|| {
			let new_id = self.next_id;

			//even if new_id fits nicely into usize32, the
			//new len() might not
			self.next_id = usize_to_32(self.next_id + 1) as usize;

			new_id as usize32
		})
	}

	fn reclaim_id(&mut self, id: usize32) {
		if id as usize == self.next_id - 1 {
			self.next_id -= 1;
		} else {
			self.reclaimed_ids.push(id);
		}
	}

	fn build_slot_path(&self, id: usize32) -> Rc<Vec<usize32>> {
		let mut element_path = Vec::with_capacity(self._diff_path.len() + 2);
		element_path.extend(self._diff_path.iter());
		element_path.push(self.field_id);
		element_path.push(id);
		Rc::new(element_path)
	}
}
