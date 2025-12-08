use crate::simulation::character;
use base::networked_types::primitive::usize32;
use base::prelude::*;

//all callbacks are guaranteed to be triggered
//in order of declaration, on the server only,
//and during an unrollbackable tick.

//called on tick id 0
pub fn on_server_start(_state: &mut SimulationState, _diff: &mut DiffSerializer<WaitForConsensus>) {}

//called after the client is added to SimulationState
pub fn on_client_connect(
	state: &mut SimulationState,
	client_id: usize32,
	_tick_id: TickID,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	character::on_client_connect(state, client_id, diff);
}

//called before the client is removed from SimulationState
pub fn on_client_disconnect(
	state: &mut SimulationState,
	id: usize32,
	_tick_id: TickID,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	character::on_client_disconnect(state, id, diff);
}
