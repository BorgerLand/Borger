use borger::interpolation::EntityInstanceBindings;
use borger::js_bindings::JSBindings;
use borger::presentation_state::{Character, SimulationOutput};

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
	let slot = tick
		.state
		.clients
		.get(tick.local_client_id)
		.unwrap()
		.as_owned()
		.unwrap()
		.character_id;

	bindings
		.entities
		.characters
		.iter()
		.find(|character| character.rs.slot_id == slot)
		.unwrap()
}
