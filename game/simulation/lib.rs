use crate::server_events::*;
use borger::prelude::*;

mod character;
mod input;
mod server_events;

pub fn init(new_client_snapshot: Vec<u8>) -> SimControllerExternals {
	init_simulation(SimulationCallbacks {
		simulation_tick,
		new_client_snapshot,
		input_merge: input::merge,
		input_validate: input::validate,
		input_predict_late: input::predict_late,
		on_server_start,
		on_client_connect,
		on_client_disconnect,
	})
}

//the deterministic-ish simulation update tick pipeline.
//this is going to run on both the server and the client.
//in a perfect world, server+client's SimulationState
//should be identical by the end of any given tick id.
//in practice this is not possible, but the closer you
//get them, the better your netcode feels
fn simulation_tick(ctx: &mut GameContext<Immediate>) {
	character::update(ctx);
}

#[cfg(feature = "client")]
pub mod old;
