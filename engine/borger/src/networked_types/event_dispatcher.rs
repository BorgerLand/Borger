use crate::diff_ser::DiffSerializer;
use crate::multiplayer_tradeoff::AnyTradeoff;
use crate::networked_types::primitive::{PrimitiveSerDes, usize32};
use crate::{DeserializeOopsy, DiffOperation};
use crate::{constructors::ConstructCollectionOrUtilityType, snapshot_serdes::SnapshotState};
use std::fmt::Debug;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

#[cfg(feature = "client")]
use {crate::interpolation::InterpolateTicks, crate::presentation::PresentTick, crate::tick::TickID};

///Event dispatcher for non-critical, unrollbackable game feel
///events: camera/shakes, footsteps, particle effects, etc. Do
///not rely on this to work 100% of the time. For example, a
///rollback can undo the event before it ever hits presentation.
#[derive(Debug)]
pub struct EventDispatcher {
	diff_path: Rc<Vec<usize32>>,
	field_id: usize32,

	#[cfg(feature = "server")]
	visibility: NetVisibility,

	version: u8,
}

//---constructors---//

impl ConstructCollectionOrUtilityType for EventDispatcher {
	fn construct(
		path: &Rc<Vec<usize32>>,
		_field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self {
		Self {
			diff_path: path.clone(),
			field_id: _field_id,

			#[cfg(feature = "server")]
			visibility,

			version: 0,
		}
	}
}

//---diff_ser---//

impl EventDispatcher {
	pub fn fire_and_forget(&mut self, diff: &mut DiffSerializer<impl AnyTradeoff>) {
		self.version = self.version.wrapping_add(1);

		let op = DiffOperation::TrackEventDispatcher;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self.diff_path) {
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self.diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
		}
	}
}

//---diff_des---//

impl EventDispatcher {
	pub(crate) fn rollback(&mut self) {
		self.version = self.version.wrapping_sub(1);
	}

	#[cfg(feature = "client")]
	pub(crate) fn rx(&mut self) {
		self.version = self.version.wrapping_add(1);
	}
}

//---snapshot---//

impl SnapshotState for EventDispatcher {
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

	fn ser_rollback_predict_remove(&self, buffer: &mut Vec<u8>) {
		self.version.ser_rollback(buffer);
	}

	fn des_rollback_predict_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		self.version = u8::des_rollback(buffer)?;
		Ok(())
	}
}

//---presentation_state---//

#[cfg(feature = "client")]
pub(crate) struct PresentationEventDispatcher(u8);

#[cfg(feature = "client")]
impl PresentTick for EventDispatcher {
	type PresentationState = PresentationEventDispatcher;

	fn clone_to_presentation(&self, _: TickID) -> Self::PresentationState {
		PresentationEventDispatcher(self.version)
	}
}

//---interpolation_state---//

#[cfg(feature = "client")]
#[repr(transparent)]
pub struct InterpolationEventDispatcher(bool);

#[cfg(feature = "client")]
impl InterpolateTicks for PresentationEventDispatcher {
	type InterpolationState = InterpolationEventDispatcher;
	fn interpolate_and_diff(
		prv: Option<&Self>,
		cur: &Self,
		_: f32,
		received_new_tick: bool,
	) -> Self::InterpolationState {
		let Some(&PresentationEventDispatcher(prv)) = prv else {
			return InterpolationEventDispatcher(false);
		};

		let cur = cur.0;
		InterpolationEventDispatcher(received_new_tick && (cur > prv || (prv - cur) > u8::MAX / 2))
	}
}
