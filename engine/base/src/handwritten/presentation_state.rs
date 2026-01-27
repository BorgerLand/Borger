use crate::ClientStateGeneric;
use crate::presentation_state::*;
use crate::simulation_state;

#[cfg(feature = "client")]
use {crate::entities::Entity, crate::entities::JSData, glam::Mat4, std::mem, web_time::Instant};

pub type ClientState = ClientStateGeneric<ClientState_owned, ClientState_remote>;

pub(crate) trait CloneToPresentationState {
	type PresentationState;

	#[cfg(feature = "client")]
	fn clone_to_presentation(&self) -> Self::PresentationState;
}

impl CloneToPresentationState for simulation_state::ClientState {
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
	pub local_client_idx: usize, //use me to index the clients array
	pub state: PresentationState,
}

//safety: mat_ptr must point to an instance of a JSData's mat field
#[cfg(feature = "client")]
pub(crate) unsafe fn get_entity_from_jsdata<'a, T: Entity>(mat_ptr: *const Mat4) -> &'a T {
	unsafe {
		let js_data = &*(mat_ptr.sub(mem::offset_of!(JSData<T>, mat)) as *const JSData<T>);
		&*js_data.ptr
	}
}
