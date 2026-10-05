use borger_plugin_sdk::diff_ser::DiffSerializer;
use borger_plugin_sdk::multiplayer_tradeoff::{AnyTradeOff, DiffSerializerToImpl};
use borger_plugin_sdk::primitive::{DeserializeOopsy, PrimitiveSerDes, usize32};
use borger_plugin_sdk::traits::{
	ConstructPlugin, DiffDeserializeCustomStruct, DiffDeserializePlugin, SnapshotState,
};
use borger_procmac::diff_operation_enum;
use std::rc::Rc;

#[cfg(feature = "server")]
use borger_plugin_sdk::NetVisibility;

#[cfg(feature = "client")]
use {
	borger_plugin_sdk::TickID,
	borger_plugin_sdk::multiplayer_tradeoff::Impl,
	borger_plugin_sdk::traits::{PresentationCollect, PresentationOutput},
};

diff_operation_enum!("EventDispatcher");

//---simulation---//

///Event dispatcher for non-critical, unrollbackable game feel
///events: camera/shakes, footsteps, particle effects, etc. Do
///not rely on this to work 100% of the time. For example, a
///rollback can undo the event before it ever hits presentation.
#[derive(Debug)]
pub struct EventDispatcher {
	diff_path: Rc<[usize32]>,
	field_id: usize32,

	#[cfg(feature = "server")]
	visibility: NetVisibility,

	version: u8,
}

//---constructors---//

impl ConstructPlugin for EventDispatcher {
	fn construct(
		path: &Rc<[usize32]>,
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
	pub fn fire_and_forget(&mut self, diff: &mut DiffSerializer<impl AnyTradeOff>) {
		self.version = self.version.wrapping_add(1);

		let op = DiffOperation::EventDispatcher;
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

impl DiffDeserializePlugin for EventDispatcher {
	fn navigate_down(&mut self, _: usize32) -> Option<&mut dyn DiffDeserializeCustomStruct> {
		None
	}

	fn des_rollback(&mut self, diff_op: u8, _: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		match DiffOperation::try_from(diff_op).map_err(|_| DeserializeOopsy)? {
			DiffOperation::EventDispatcher => {
				self.version = self.version.wrapping_sub(1);
			}
		};

		Ok(())
	}

	#[cfg(feature = "client")]
	fn des_rx(
		&mut self,
		diff_op: u8,
		_: &mut std::vec::IntoIter<u8>,
		_: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy> {
		match DiffOperation::try_from(diff_op).map_err(|_| DeserializeOopsy)? {
			DiffOperation::EventDispatcher => {
				self.version = self.version.wrapping_add(1);
			}
		};

		Ok(())
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

//---presentation_collect---//

#[cfg(feature = "client")]
pub struct PresentationCollectEventDispatcher(u8);

#[cfg(feature = "client")]
impl PresentationCollect for EventDispatcher {
	type PresentationCollect = PresentationCollectEventDispatcher;

	fn presentation_collect(&self, _: TickID) -> Self::PresentationCollect {
		PresentationCollectEventDispatcher(self.version)
	}
}

//---presentation_output---//

#[cfg(feature = "client")]
impl PresentationOutput for PresentationCollectEventDispatcher {
	type PresentationOutput = bool;
	fn presentation_output(
		prv: Option<&Self>,
		cur: &Self,
		_: f32,
		received_new_tick: bool,
	) -> Self::PresentationOutput {
		let Some(&PresentationCollectEventDispatcher(prv)) = prv else {
			return false;
		};

		let cur = cur.0;
		received_new_tick && (cur > prv || (prv - cur) > u8::MAX / 2)
	}
}
