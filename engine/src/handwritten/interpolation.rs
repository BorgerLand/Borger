use crate::interpolation::*;
use crate::presentation;
use crate::scope::Scope;
use borger_plugin_sdk::primitive::usize32;
use borger_plugin_sdk::traits::InterpolateTicks;

pub type Client = Scope<ClientOwned, ClientRemote>;

impl InterpolateTicks for presentation::Client {
	type InterpolationOutput = Client;
	fn interpolate_and_diff(
		prv: Option<&Self>,
		cur: &Self,
		amount: f32,
		received_new_tick: bool,
	) -> Self::InterpolationOutput {
		match cur {
			Self::Owned(cur) => Client::Owned(InterpolateTicks::interpolate_and_diff(
				prv.map(|prv| prv.as_owned().unwrap()),
				cur,
				amount,
				received_new_tick,
			)),
			Self::Remote(cur) => {
				if let Some(presentation::Client::Owned(prv)) = prv {
					//special case: received a new client id after reconnecting
					//to server. the previously owned client is now a stale
					//remote client that the server hasn't timed out yet
					Client::Remote(presentation::ClientRemote::interpolate_and_diff(
						Some(prv),
						cur,
						amount,
						received_new_tick,
					))
				} else {
					Client::Remote(presentation::ClientRemote::interpolate_and_diff(
						prv.map(|prv| prv.as_remote().unwrap()),
						cur,
						amount,
						received_new_tick,
					))
				}
			}
		}
	}
}

pub struct InterpolationContext {
	pub local_client_id: usize32,
	pub output: InterpolationOutput,
}
