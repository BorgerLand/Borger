use crate::presentation::*;
use crate::scope::Scope;
use crate::simulation;
use borger_plugin_sdk::TickID;
use borger_plugin_sdk::primitive::usize32;
use borger_plugin_sdk::traits::PresentTick;
use web_time::Instant;

pub struct PresentationContext {
	pub time: Instant,
	pub local_client_id: usize32,
	pub output: PresentationOutput,
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
