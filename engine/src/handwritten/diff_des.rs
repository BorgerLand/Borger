use crate::simulation::{Client, State};
use borger_plugin_sdk::DiffOperation;
use borger_plugin_sdk::primitive::{DeserializeOopsy, PrimitiveSerDes, SliceSerDes, usize32};
use borger_plugin_sdk::traits::{DiffDeserializeCustomStruct, DiffDeserializePlugin};
use borger_procmac::get_plugin_diff_op_range;
use std::collections::VecDeque;

#[cfg(feature = "client")]
use {borger_plugin_sdk::diff_ser::DiffSerializer, borger_plugin_sdk::multiplayer_tradeoff::Impl, std::vec};

impl DiffDeserializeCustomStruct for Client {
	fn set_field_rollback(
		&mut self,
		field_id: usize32,
		buffer: &mut Vec<u8>,
	) -> Result<(), DeserializeOopsy> {
		match self {
			Self::Owned(client) => client.set_field_rollback(field_id, buffer),
			Self::Remote(client) => client.set_field_rollback(field_id, buffer),
		}
	}

	#[cfg(feature = "client")]
	fn set_field_rx(
		&mut self,
		field_id: usize32,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy> {
		match self {
			Self::Owned(client) => client.set_field_rx(field_id, buffer, diff),
			Self::Remote(client) => client.set_field_rx(field_id, buffer, diff),
		}
	}

	fn get_plugin(&mut self, field_id: usize32) -> Result<&mut dyn DiffDeserializePlugin, DeserializeOopsy> {
		match self {
			Self::Owned(client) => client.get_plugin(field_id),
			Self::Remote(client) => client.get_plugin(field_id),
		}
	}
}

//revert predictions
pub fn des_rollback(state: &mut State, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
	//safety: the current iteration of the loop may
	//only dereference the top element of the stack.
	//each consecutive element has a shorter lifetime
	//than the previous element
	let mut diff_path_stack: Vec<*mut dyn DiffDeserializeCustomStruct> = Vec::new();
	let root_path = state as *mut dyn DiffDeserializeCustomStruct;
	let mut cur_path = root_path;

	loop {
		let cur_nav_state = unsafe { cur_path.as_mut() }.unwrap();

		let diff_op = u8::des_rollback(buffer)?;
		if get_plugin_diff_op_range!().contains(&diff_op) {
			let field_id = usize32::des_rollback(buffer)?;
			let Ok(field) = cur_nav_state.get_plugin(field_id) else {
				return Err(DeserializeOopsy);
			};

			field.des_rollback(diff_op, buffer)?;
		} else {
			match DiffOperation::try_from(diff_op) {
				Ok(DiffOperation::SetPrimitive) => {
					let field_id = usize32::des_rollback(buffer)?;
					cur_nav_state.set_field_rollback(field_id, buffer)?;
				}

				Ok(DiffOperation::NavigateUp) => {
					let nav_up_len = u8::des_rollback(buffer)?;
					for _ in 0..nav_up_len {
						cur_path = diff_path_stack.pop().ok_or(DeserializeOopsy)?;
					}
				}
				Ok(DiffOperation::NavigateDown) => {
					let nav_down_len = u8::des_rollback(buffer)? as usize32;
					let mut nav_down = VecDeque::from(<[usize32]>::des_rollback(nav_down_len * 2, buffer)?);
					while !nav_down.is_empty() {
						let field_id = nav_down.pop_front().ok_or(DeserializeOopsy)?;
						let element_id = nav_down.pop_front().ok_or(DeserializeOopsy)?;

						let Ok(field) = unsafe { cur_path.as_mut() }.unwrap().get_plugin(field_id) else {
							return Err(DeserializeOopsy);
						};

						diff_path_stack.push(cur_path);
						cur_path = field.navigate_down(element_id).ok_or(DeserializeOopsy)?;
					}
				}
				Ok(DiffOperation::NavigateReset) => {
					diff_path_stack.clear();
					cur_path = root_path;
				}

				Ok(DiffOperation::RollbackTickSeparator) => break, //done
				Err(_) => return Err(DeserializeOopsy),
			};
		}
	}

	Ok(())
}

//apply authoritative state changes from server
#[cfg(feature = "client")]
pub fn des_rx_state(
	state: &mut State,
	buffer: &mut vec::IntoIter<u8>,
	diff: &mut DiffSerializer<Impl>,
) -> Result<(), DeserializeOopsy> {
	//safety: the current iteration of the loop may
	//only dereference the top element of the stack.
	//each consecutive element has a shorter lifetime
	//than the previous element
	let mut diff_path_stack: Vec<*mut dyn DiffDeserializeCustomStruct> = Vec::new();
	let root_path = state as *mut dyn DiffDeserializeCustomStruct;
	let mut cur_path = root_path;

	loop {
		let cur_nav_state = unsafe { cur_path.as_mut() }.unwrap();

		let diff_op = match u8::des_rx(buffer) {
			Ok(diff_op) => diff_op,
			Err(_) => break, //only possible on an empty buffer, implying it's done
		};

		if get_plugin_diff_op_range!().contains(&diff_op) {
			let field_id = usize32::des_rx(buffer)?;
			let Ok(field) = cur_nav_state.get_plugin(field_id) else {
				return Err(DeserializeOopsy);
			};

			field.des_rx(diff_op, buffer, diff)?;
		} else {
			match DiffOperation::try_from(diff_op) {
				Ok(DiffOperation::SetPrimitive) => {
					let field_id = usize32::des_rx(buffer)?;
					cur_nav_state.set_field_rx(field_id, buffer, diff)?;
				}

				Ok(DiffOperation::NavigateUp) => {
					let nav_up_len = u8::des_rx(buffer)?;
					for _ in 0..nav_up_len {
						cur_path = diff_path_stack.pop().ok_or(DeserializeOopsy)?;
					}
				}
				Ok(DiffOperation::NavigateDown) => {
					let nav_down_len = u8::des_rx(buffer)? as usize32;
					let mut nav_down: VecDeque<usize32> =
						<[usize32]>::des_rx(nav_down_len * 2, buffer)?.into();
					while !nav_down.is_empty() {
						let field_id = nav_down.pop_front().ok_or(DeserializeOopsy)?;
						let element_id = nav_down.pop_front().ok_or(DeserializeOopsy)?;

						let Ok(field) = unsafe { cur_path.as_mut() }.unwrap().get_plugin(field_id) else {
							return Err(DeserializeOopsy);
						};

						diff_path_stack.push(cur_path);
						cur_path = field.navigate_down(element_id).ok_or(DeserializeOopsy)?;
					}
				}
				Ok(DiffOperation::NavigateReset) => {
					diff_path_stack.clear();
					cur_path = root_path;
				}

				Ok(DiffOperation::RollbackTickSeparator) => {
					return Err(DeserializeOopsy);
				}
				Err(_) => return Err(DeserializeOopsy),
			};
		}
	}

	Ok(())
}
