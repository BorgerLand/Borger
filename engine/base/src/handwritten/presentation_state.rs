use crate::ClientStateGeneric;
use crate::presentation_state::*;
use crate::simulation_state;

#[cfg(feature = "client")]
use {
	crate::interpolation::{Entity, EntityInstanceRSBindings},
	crate::networked_types::primitive::usize32,
	glam::Mat4,
	std::mem,
	web_time::Instant,
};

pub type ClientState = ClientStateGeneric<ClientState_owned, ClientState_remote>;

pub(crate) trait CloneToPresentationState {
	#[cfg(feature = "client")]
	type PresentationState;

	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState;
}

impl CloneToPresentationState for simulation_state::ClientState {
	#[cfg(feature = "client")]
	type PresentationState = ClientState;

	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState {
		match self {
			simulation_state::ClientState::Owned(client) => {
				ClientState::Owned(client.clone_to_presentation())
			}
			simulation_state::ClientState::Remote(client) => {
				ClientState::Remote(client.clone_to_presentation())
			}
		}
	}
}

#[cfg(feature = "client")]
pub struct SimulationOutput {
	pub time: Instant,
	pub local_client_id: usize32,
	pub state: PresentationState,
}

//safety: rs_ptr must point to a EntityInstanceRSBindings's mat field
#[cfg(feature = "client")]
pub(crate) unsafe fn get_presentation_from_mat<'a, T: Entity>(rs_ptr: *const Mat4) -> &'a T {
	let offset = mem::offset_of!(EntityInstanceRSBindings<T>, mat);
	unsafe {
		let bindings_rs = &*(rs_ptr.byte_sub(offset) as *const EntityInstanceRSBindings<T>);
		bindings_rs.interpolated()
	}
}
