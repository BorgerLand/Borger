use crate::networked_types::collections::slotmap::SlotMapDynCompat;
use crate::networked_types::primitive::{PrimitiveSerDes, SliceSerDes, usize32};
use crate::simulation_state::{ClientState, SimulationState};
use crate::{DeserializeOopsy, DiffOperation};
use std::collections::VecDeque;

#[cfg(feature = "client")]
use {crate::context::Impl, crate::diff_ser::DiffSerializer, std::vec};

pub(crate) trait DiffDeserializeState {
	fn set_field_rollback(&mut self, field_id: usize32, buffer: &mut Vec<u8>)
	-> Result<(), DeserializeOopsy>;

	#[cfg(feature = "client")]
	fn set_field_rx(
		&mut self,
		field_id: usize32,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy>;

	//collections+utilities
	fn get_slotmap(&mut self, field_id: usize32) -> Result<&mut dyn SlotMapDynCompat, DeserializeOopsy>;
}

impl DiffDeserializeState for ClientState {
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

	fn get_slotmap(&mut self, field_id: usize32) -> Result<&mut dyn SlotMapDynCompat, DeserializeOopsy> {
		match self {
			Self::Owned(client) => client.get_slotmap(field_id),
			Self::Remote(client) => client.get_slotmap(field_id),
		}
	}
}

//revert predictions
pub fn des_rollback(state: &mut SimulationState, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
	//safety: the current iteration of the loop may
	//only dereference the top element of the stack.
	//each consecutive element has a shorter lifetime
	//than the previous element
	let mut diff_path_stack: Vec<*mut dyn DiffDeserializeState> = Vec::new();
	let root_path = state as *mut dyn DiffDeserializeState;
	let mut cur_path = root_path;

	loop {
		let cur_nav_state = unsafe { cur_path.as_mut() }.unwrap();

		match DiffOperation::des_rollback(buffer) {
			Ok(DiffOperation::TrackPrimitive) => {
				let field_id = usize32::des_rollback(buffer)?;
				cur_nav_state.set_field_rollback(field_id, buffer)?;
			}
			Ok(DiffOperation::TrackSlotMapAdd) => {
				let field_id = usize32::des_rollback(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rollback_add();
			}
			Ok(DiffOperation::TrackSlotMapRemove) => {
				let field_id = usize32::des_rollback(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rollback_remove(buffer)?;
			}
			Ok(DiffOperation::TrackSlotMapClear) => {
				let field_id = usize32::des_rollback(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rollback_clear(buffer)?;
			}

			Ok(DiffOperation::NavigateUp) => {
				let nav_up_len = u8::des_rollback(buffer)?;
				for _ in 0..nav_up_len {
					cur_path = diff_path_stack.pop().ok_or(DeserializeOopsy::PathNotFound)?;
				}
			}
			Ok(DiffOperation::NavigateDown) => {
				let nav_down_len = u8::des_rollback(buffer)? as usize32;
				let mut nav_down: VecDeque<usize32> =
					<[usize32]>::des_rollback(nav_down_len * 2, buffer)?.into();
				while nav_down.len() > 0 {
					let field_id = nav_down.pop_front().ok_or(DeserializeOopsy::PathNotFound)?;
					let element_id = nav_down.pop_front().ok_or(DeserializeOopsy::PathNotFound)?;

					diff_path_stack.push(cur_path);
					cur_path = unsafe { cur_path.as_mut() }
						.unwrap()
						.get_slotmap(field_id)?
						.get_des(element_id)
						.ok_or(DeserializeOopsy::PathNotFound)?;
				}
			}
			Ok(DiffOperation::NavigateReset) => {
				diff_path_stack.clear();
				cur_path = root_path;
			}

			Ok(DiffOperation::RollbackTickSeparator) => break, //done
			Err(oops) => return Err(oops),
		};
	}

	Ok(())
}

//apply authoritative state changes from server
#[cfg(feature = "client")]
pub fn des_rx_state(
	state: &mut SimulationState,
	buffer: &mut vec::IntoIter<u8>,
	diff: &mut DiffSerializer<Impl>,
) -> Result<(), DeserializeOopsy> {
	//safety: the current iteration of the loop may
	//only dereference the top element of the stack.
	//each consecutive element has a shorter lifetime
	//than the previous element
	let mut diff_path_stack: Vec<*mut dyn DiffDeserializeState> = Vec::new();
	let root_path = state as *mut dyn DiffDeserializeState;
	let mut cur_path = root_path;

	loop {
		let cur_nav_state = unsafe { cur_path.as_mut() }.unwrap();

		match DiffOperation::des_rx(buffer) {
			Ok(DiffOperation::TrackPrimitive) => {
				let field_id = usize32::des_rx(buffer)?;
				cur_nav_state.set_field_rx(field_id, buffer, diff)?;
			}
			Ok(DiffOperation::TrackSlotMapAdd) => {
				let field_id = usize32::des_rx(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rx_add(diff);
			}
			Ok(DiffOperation::TrackSlotMapRemove) => {
				let field_id = usize32::des_rx(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rx_remove(buffer, diff)?;
			}
			Ok(DiffOperation::TrackSlotMapClear) => {
				let field_id = usize32::des_rx(buffer)?;
				cur_nav_state.get_slotmap(field_id)?.rx_clear(diff);
			}

			Ok(DiffOperation::NavigateUp) => {
				let nav_up_len = u8::des_rx(buffer)?;
				for _ in 0..nav_up_len {
					cur_path = diff_path_stack.pop().ok_or(DeserializeOopsy::PathNotFound)?;
				}
			}
			Ok(DiffOperation::NavigateDown) => {
				let nav_down_len = u8::des_rx(buffer)? as usize32;
				let mut nav_down: VecDeque<usize32> = <[usize32]>::des_rx(nav_down_len * 2, buffer)?.into();
				while nav_down.len() > 0 {
					let field_id = nav_down.pop_front().ok_or(DeserializeOopsy::PathNotFound)?;
					let element_id = nav_down.pop_front().ok_or(DeserializeOopsy::PathNotFound)?;

					diff_path_stack.push(cur_path);
					cur_path = unsafe { cur_path.as_mut() }
						.unwrap()
						.get_slotmap(field_id)?
						.get_des(element_id)
						.ok_or(DeserializeOopsy::PathNotFound)?;
				}
			}
			Ok(DiffOperation::NavigateReset) => {
				diff_path_stack.clear();
				cur_path = root_path;
			}

			Err(DeserializeOopsy::BufferUnderflow) => break, //done
			Ok(DiffOperation::RollbackTickSeparator) => {
				return Err(DeserializeOopsy::CorruptDiffOperation);
			}
			Err(oops) => return Err(oops),
		};
	}

	Ok(())
}
