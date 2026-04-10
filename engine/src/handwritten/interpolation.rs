#[cfg(feature = "client")]
use {
	crate::Scope, crate::interpolation::*, crate::networked_types::primitive::usize32, crate::presentation,
};

pub trait Interpolate: Copy {
	fn interpolate(prv: Self, cur: Self, amount: f32) -> Self;
}

//trait exists to fire events when some change occurs between
//ticks (eg. for slotmaps, adding+removing slots). prv has to
//be an option in order to fire initial collection add/remove
//events, otherwise no change would be detected
#[cfg(feature = "client")]
pub trait InterpolateTicks {
	type InterpolationState;
	fn interpolate_and_diff(
		prv: Option<&Self>,
		cur: &Self,
		amount: f32,
		received_new_tick: bool,
	) -> Self::InterpolationState;
}

#[cfg(feature = "client")]
pub type Client = Scope<ClientOwned, ClientRemote>;

#[cfg(feature = "client")]
impl InterpolateTicks for presentation::Client {
	type InterpolationState = Client;
	fn interpolate_and_diff(
		prv: Option<&Self>,
		cur: &Self,
		amount: f32,
		receive_new_tick: bool,
	) -> Self::InterpolationState {
		match cur {
			Self::Owned(cur) => Client::Owned(InterpolateTicks::interpolate_and_diff(
				prv.map(|prv| prv.as_owned().unwrap()),
				cur,
				amount,
				receive_new_tick,
			)),
			Self::Remote(cur) => Client::Remote(InterpolateTicks::interpolate_and_diff(
				prv.map(|prv| prv.as_remote().unwrap()),
				cur,
				amount,
				receive_new_tick,
			)),
		}
	}
}

#[cfg(feature = "client")]
pub struct InterpolationOutput {
	pub local_client_id: usize32,
	pub state: InterpolationState,
}
