use crate::DeserializeOopsy;
use crate::diff_ser::DiffSerializer;
use crate::multiplayer_tradeoff::Immediate;
use crate::networked_types::collections::slotmap::SlotMap;
use crate::networked_types::primitive::usize32;
use crate::presentation_state::CloneToPresentationState;
use crate::simulation_state::ClientState;
use crate::tick::TickInfo;
use crate::{constructors::ConstructCollectionOrUtilityType, snapshot_serdes::SnapshotState};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::rc::Rc;

#[cfg(feature = "server")]
use {
	crate::DiffOperation, crate::NetVisibility, crate::networked_types::primitive::PrimitiveSerDes,
	crate::simulation_state::InputStateAge, std::marker::PhantomData,
};

#[cfg(feature = "client")]
use {
	crossbeam_channel::{
		Receiver as SyncReceiver, Sender as SyncSender, unbounded as sync_unbounded_channel,
	},
	std::vec,
};

///Event emitter for non-critical, opportunistically
///predicted, unrollbackable game feel events: camera
///shakes, footsteps, particle effects, etc. Solves
///the problem of the presentation loop not providing
///any guarantee to render every single simulation
///tick.
#[derive(Debug)]
pub struct HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	_diff_path: Rc<Vec<usize32>>,

	#[cfg(feature = "server")]
	field_id: usize32,

	#[cfg(feature = "server")]
	visibility: NetVisibility,

	#[cfg(feature = "server")]
	events: PhantomData<T>,
	#[cfg(feature = "client")]
	events: (SyncSender<T>, SyncReceiver<T>),
}

//---presentation_state---//

impl<T> CloneToPresentationState for HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	#[cfg(feature = "client")]
	type PresentationState = SyncReceiver<T>;

	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState {
		self.events.1.clone()
	}
}

//---constructors---//

impl<T> ConstructCollectionOrUtilityType for HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	fn construct(
		path: &Rc<Vec<usize32>>,
		_field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self {
		Self {
			_diff_path: path.clone(),

			#[cfg(feature = "server")]
			field_id: _field_id,

			#[cfg(feature = "server")]
			visibility,

			#[cfg(feature = "server")]
			events: PhantomData,
			#[cfg(feature = "client")]
			events: sync_unbounded_channel(),
		}
	}
}

//---diff_ser---//

impl<T> HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	pub fn emit(
		&self,
		event: T,
		#[allow(unused_variables)] tick: &TickInfo,
		#[allow(unused_variables)] triggering_client: Option<(&SlotMap<ClientState>, usize32)>,
		#[allow(unused_variables)] diff: &mut DiffSerializer<Immediate>,
	) {
		#[cfg(feature = "server")]
		{
			if let Some((clients, triggering_client)) = triggering_client {
				let op = DiffOperation::TrackHapticPrediction;
				let diff = diff.to_impl();

				for buffer in diff.ser_tx_begin(
					&self._diff_path,
					self.visibility,
					Some(|tx_client_id| {
						tx_client_id != triggering_client
							&& clients
								.get(tx_client_id)
								.unwrap()
								.as_owned()
								.unwrap()
								.input
								.get()
								.age == InputStateAge::Fresh
					}),
				) {
					op.ser_tx(buffer);
					self.field_id.ser_tx(buffer);

					//postcard::to_extend(&event, buffer);
					postcard::to_io(&event, buffer).unwrap();
				}
			}
		}

		#[cfg(feature = "client")]
		if tick.is_fresh() {
			self.events.0.send(event).unwrap();
		}
	}
}

//---diff_des---//

//wrapper to eliminate haptic prediction's generics and allow
//HapticPredictionEmitterDynCompat trait to be dyn compatible

#[cfg(feature = "client")]
pub(crate) trait HapticPredictionEmitterDynCompat {
	fn rx(&self, buffer: &mut vec::IntoIter<u8>) -> Result<(), DeserializeOopsy>;
}

#[cfg(feature = "client")]
impl<T> HapticPredictionEmitterDynCompat for HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	fn rx(&self, buffer: &mut vec::IntoIter<u8>) -> Result<(), DeserializeOopsy> {
		let before = buffer.as_slice();
		let (event, after) =
			postcard::take_from_bytes(before).map_err(|_| DeserializeOopsy::CorruptHapticPrediction)?;

		let consumed = before.len() - after.len();
		if consumed == 0 {
			//sanity check: should never happen (unless t is zst?)
			return Err(DeserializeOopsy::CorruptHapticPrediction);
		}

		buffer.nth(consumed - 1);
		self.events.0.send(event).unwrap();
		Ok(())
	}
}

//---snapshot---//

impl<T> SnapshotState for HapticPredictionEmitter<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, _: usize32, _: &mut Vec<u8>) {}

	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		_: usize32,
		_: &mut impl Iterator<Item = u8>,
	) -> Result<(), DeserializeOopsy> {
		Ok(())
	}

	fn ser_rollback_predict_remove(&self, _: &mut Vec<u8>) {}

	fn des_rollback_predict_remove(&mut self, _: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		Ok(())
	}
}
