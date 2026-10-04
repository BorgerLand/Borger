use crate::presentation_collect::*;
use crate::scope::Scope;
use crate::simulation;
use borger_plugin_sdk::TickID;
use borger_plugin_sdk::primitive::usize32;
use borger_plugin_sdk::traits::PresentationCollect;
use web_time::Instant;

pub struct PresentationCollectContext {
	pub time: Instant,
	pub local_client_id: usize32,
	pub output: PresentationCollectState,
}

pub(crate) type Client = Scope<ClientOwned, ClientRemote>;

impl PresentationCollect for simulation::Client {
	type PresentationCollect = Client;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationCollect {
		match self {
			simulation::Client::Owned(client) => Client::Owned(client.clone_to_presentation(tick)),
			simulation::Client::Remote(client) => Client::Remote(client.clone_to_presentation(tick)),
		}
	}
}
