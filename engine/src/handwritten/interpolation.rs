#[cfg(feature = "client")]
use {
	crate::Scope, crate::interpolation::*, crate::networked_types::primitive::usize32, crate::presentation,
};

///Classic lerp/slerp helper for various simple math primitives
pub trait Interpolate: Copy {
	fn interpolate(prv: Self, cur: Self, amount: f32) -> Self;
}

//trait exists to fire events when some change occurs between
//ticks (eg. for slotmaps, adding+removing slots). prv has to
//be an option in order to fire initial collection add/remove
//events, otherwise no change would be detected
#[cfg(feature = "client")]
pub trait InterpolateTicks<Prv = Self> {
	type InterpolationOutput;
	fn interpolate_and_diff(
		prv: Option<&Prv>,
		cur: &Self,
		amount: f32,
		received_new_tick: bool,
	) -> Self::InterpolationOutput;
}

#[cfg(feature = "client")]
pub type Client = Scope<ClientOwned, ClientRemote>;

#[cfg(feature = "client")]
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

#[cfg(feature = "client")]
pub struct InterpolationContext {
	pub local_client_id: usize32,
	pub output: InterpolationOutput,
}
