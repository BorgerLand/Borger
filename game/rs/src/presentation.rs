use base::interpolation::EntityInstanceBindings;
use base::prelude::*;
use base::presentation_state::Character;

pub mod camera;
pub mod pipeline;

pub fn on_client_start(_bindings: &mut JSBindings) {}

pub fn get_local_entity<'a>(
	tick: &SimulationOutput,
	bindings: &'a JSBindings,
) -> &'a EntityInstanceBindings<Character> {
	//treasure hunt for the position of this client's character.
	//ideally instead of this o(n) search, SlotMap's impl for
	//CloneToPresentationState::PresentationState would return a
	//standard SlotMap collection decoupled from networking. that
	//would allow a quick hashmap lookup. however the slotmap
	//file needs a refactor first
	let slot = tick.state.clients[tick.local_client_idx].1.as_owned().unwrap().id;
	bindings
		.entities
		.characters
		.iter()
		.find(|character| character.rs.slot_id == slot)
		.unwrap()
}
