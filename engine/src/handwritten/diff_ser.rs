use borger_plugin_sdk::DiffOperation;
use borger_plugin_sdk::diff_ser::DiffSerializer;
use borger_plugin_sdk::multiplayer_tradeoff::Impl;
use borger_plugin_sdk::primitive::{PrimitiveSerDes, usize32};
use std::rc::Rc;

#[cfg(feature = "server")]
use borger_plugin_sdk::NetVisibility;

//note field_id is technically part of the path but
//is passed as a separate parameter for optimization
//purposes (avoid having a vec for every single
//field, avoid changing path when writing to
//multiple fields on the same struct).
//
//rollback_prv_value: represents data that will be
//rolled back. the state's previous value will be
//written
//
//tx_new_value: represents data that will be sent
//over the wire. the state's new value will be
//written. the visibility and path arguments will
//determine who it's sent to
pub fn ser_sim_primitive<T: PrimitiveSerDes>(
	diff: &mut DiffSerializer<Impl>,
	path: &Rc<Vec<usize32>>,
	field_id: usize32,
	rollback_prv_value: T,

	#[cfg(feature = "server")] visibility: NetVisibility,
	#[cfg(feature = "server")] tx_new_value: T,
) {
	let op = DiffOperation::SetPrimitive;

	if let Some(buffer) = diff.ser_rollback_begin(path) {
		rollback_prv_value.ser_rollback(buffer);
		field_id.ser_rollback(buffer);
		op.ser_rollback(buffer);
	}

	#[cfg(feature = "server")]
	for buffer in diff.ser_tx_begin(path, visibility) {
		op.ser_tx(buffer);
		field_id.ser_tx(buffer);
		tx_new_value.ser_tx(buffer);
	}
}

//slimmed down version of ser_sim_primitive specifically for
//clients writing to their input states, which only have
///primitive types, can only do DiffOperation::SetPrimitive,
//and never roll back
#[cfg(feature = "client")]
pub fn ser_input_primitive<T: PrimitiveSerDes>(
	diff: &mut DiffSerializer<Impl>,
	field_id: usize32,
	tx_new_value: T,
) {
	let buffer = diff.ser_tx_begin();
	field_id.ser_tx(buffer);
	tx_new_value.ser_tx(buffer);
}
