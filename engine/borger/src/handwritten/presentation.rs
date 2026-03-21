use crate::Scope;
use crate::interpolation::InterpolateTicks;
use crate::networked_types::primitive::usize32;
use crate::presentation::*;
use crate::simulation_state;
use crate::tick::TickID;
use web_time::Instant;

pub struct SimulationOutput {
	pub time: Instant,
	pub local_client_id: usize32,
	pub state: PresentationState,
}

pub(crate) trait PresentTick {
	type PresentationState: InterpolateTicks;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationState;
}

pub(crate) type Client = Scope<ClientOwned, ClientRemote>;
impl PresentTick for simulation_state::Client {
	type PresentationState = Client;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationState {
		match self {
			simulation_state::Client::Owned(client) => Client::Owned(client.clone_to_presentation(tick)),
			simulation_state::Client::Remote(client) => Client::Remote(client.clone_to_presentation(tick)),
		}
	}
}
