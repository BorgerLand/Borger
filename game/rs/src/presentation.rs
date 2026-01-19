use base::entities::InterpolatedEntityInstance;
use base::prelude::*;

pub mod camera;
pub mod pipeline;

pub fn on_client_start(_bindings: &mut JSBindings) {}

pub fn get_local_entity<'a>(
	tick: &SimulationOutput,
	bindings: &'a JSBindings,
) -> &'a InterpolatedEntityInstance {
	//treasure hunt for the position of this client's character
	bindings
		.entities
		.characters
		.obj
		.get(&tick.state.clients[tick.local_client_idx].1.as_owned().unwrap().id)
		.unwrap()
}
