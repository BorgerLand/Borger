use crate::presentation_collect;
use crate::presentation_output::*;
use crate::scope::Scope;
use borger_plugin_sdk::primitive::usize32;
use borger_plugin_sdk::traits::PresentationOutput;

pub struct PresentationOutputContext {
	pub interpolation_alpha: f32,
	pub received_new_tick: bool,
	pub local_client_id: usize32,
	pub output: PresentationOutputState,
}

pub type Client = Scope<ClientOwned, ClientRemote>;

impl PresentationOutput for presentation_collect::Client {
	type PresentationOutput = Client;
	fn presentation_output(
		prv: Option<&Self>,
		cur: &Self,
		interpolation_alpha: f32,
		received_new_tick: bool,
	) -> Self::PresentationOutput {
		match cur {
			Self::Owned(cur) => Client::Owned(PresentationOutput::presentation_output(
				prv.map(|prv| prv.as_owned().unwrap()),
				cur,
				interpolation_alpha,
				received_new_tick,
			)),
			Self::Remote(cur) => {
				if let Some(presentation_collect::Client::Owned(prv)) = prv {
					//special case: received a new client id after reconnecting
					//to server. the previously owned client is now a stale
					//remote client that the server hasn't timed out yet
					Client::Remote(presentation_collect::ClientRemote::presentation_output(
						Some(prv),
						cur,
						interpolation_alpha,
						received_new_tick,
					))
				} else {
					Client::Remote(presentation_collect::ClientRemote::presentation_output(
						prv.map(|prv| prv.as_remote().unwrap()),
						cur,
						interpolation_alpha,
						received_new_tick,
					))
				}
			}
		}
	}
}
