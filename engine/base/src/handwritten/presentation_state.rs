use crate::ClientStateGeneric;
use crate::presentation_state::*;
use crate::simulation_state;

#[cfg(feature = "client")]
use {crate::simulation_controller::SimulationInternals, std::sync::atomic::Ordering, web_time::Instant};

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
pub struct PresentationTick {
	pub time: Instant,
	pub local_client_idx: usize, //use me to index the clients array
	pub state: PresentationState,
}

#[cfg(feature = "client")]
pub(crate) fn output_presentation(sim: &mut SimulationInternals) {
	sim.output_sender.store(
		Some(std::boxed::Box::new(PresentationTick {
			time: sim.ctx.tick.get_now(),
			local_client_idx: sim.ctx.state.clients.random_access(sim.local_client_id).unwrap(),
			state: sim.ctx.state.clone_to_presentation(),
		})),
		Ordering::AcqRel,
	);
}
