use borger::prelude::*;

pub mod character;
pub mod input;

pub fn init() -> SimulationInitOptions {
	SimulationInitOptions {
		init_static_level_geom: None,
		simulation_loop,
		input_merge: input::merge,
		input_validate: input::validate,
		input_server_predict_late: input::server_predict_late,
		input_client_predict_late: input::client_predict_late,
		on_server_start,
		on_client_connect,
		on_client_disconnect,
	}
}

//the deterministic-ish simulation update tick pipeline.
//this is going to run on both the server and the client.
//in a perfect world, server+client's SimulationState
//should be identical by the end of any given tick id.
//in practice this is not possible due to latency, but
//the closer you get them, the better your game feels
fn simulation_loop(ctx: &mut GameContext<Immediate>) {
	character::update(ctx);
}

//called on tick id 0
#[server]
pub fn on_server_start(_state: &mut SimulationState, _diff: &mut DiffSerializer<WaitForConsensus>) {}

//called after the client is added to SimulationState
#[server]
pub fn on_client_connect(
	state: &mut SimulationState,
	client_id: usize32,
	_tick_id: TickID,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	character::on_client_connect(state, client_id, diff);
}

//called before the client is removed from SimulationState
#[server]
pub fn on_client_disconnect(
	state: &mut SimulationState,
	client_id: usize32,
	_tick_id: TickID,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	character::on_client_disconnect(state, client_id, diff);
}
