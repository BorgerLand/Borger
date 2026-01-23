use crate::simulation::{character, physics_demo, physstep};
use base::prelude::*;

//the deterministic-ish simulation update tick pipeline.
//this is going to run on both the server and the client.
//in a perfect world, server+client's SimulationState
//should be identical by the end of any given tick id.
//in practice this is not possible, but the closer you
//get them, the better your netcode feels. recommend
//switching /.vscode/settings.json to server mode while
//working in here
pub fn simulation_tick(ctx: &mut GameContext<Immediate>) {
	//set the stage for physstep
	character::update_pre_physstep(ctx);
	physics_demo::update_pre_physstep(ctx);

	physstep::update(ctx);

	//use the results of physstep
	character::update_post_physstep(ctx);
	physics_demo::update_post_physstep(ctx);
}
