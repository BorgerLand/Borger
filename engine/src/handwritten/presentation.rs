use crate::Scope;
use crate::interpolation::InterpolateTicks;
use crate::networked_types::primitive::usize32;
use crate::presentation::*;
use crate::simulation;
use crate::tick::TickID;
use web_time::Instant;

pub struct PresentationContext {
	pub time: Instant,
	pub local_client_id: usize32,
	pub output: PresentationOutput,
}

pub(crate) trait PresentTick {
	type PresentationOutput: InterpolateTicks;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationOutput;
}

pub(crate) type Client = Scope<ClientOwned, ClientRemote>;
impl PresentTick for simulation::Client {
	type PresentationOutput = Client;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationOutput {
		match self {
			simulation::Client::Owned(client) => Client::Owned(client.clone_to_presentation(tick)),
			simulation::Client::Remote(client) => Client::Remote(client.clone_to_presentation(tick)),
		}
	}
}
