use borger::networked_types::primitive::usize32;
use borger::prelude::*;
use glam::{Quat, Vec3};

const SPEED: f32 = 6.0; //units/sec

#[server]
pub fn on_client_connect(
	state: &mut SimulationState,
	client_id: usize32,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	let client = state.clients.get_mut(client_id).unwrap().as_owned_mut().unwrap();
	let character = state.characters.add(diff).0;
	client.set_character_id(character, diff);
}

#[server]
pub fn on_client_disconnect(
	state: &mut SimulationState,
	client_id: usize32,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	let client = state.clients.get_mut(client_id).unwrap().as_owned_mut().unwrap();
	let character = client.get_character_id();
	state.characters.remove(character, diff).unwrap();
}

pub fn update(ctx: &mut GameContext<Immediate>) {
	//remember: the server "owns" all client objects.
	//a locally running client only owns their own client
	//object. the "input" field has owner visibility,
	//so effectively the server simulates all players
	//while each client only simulates their own. the
	//server then informs all players of where all the
	//other "remote" players are
	for client in ctx.state.clients.values_mut() {
		if let ClientState::Owned(client) = client {
			let character = ctx.state.characters.get_mut(client.get_character_id()).unwrap();
			let input = client.input.get();

			//if the input was predicted then don't bother moving until
			//it arrives. otherwise there's a risk of running off a cliff
			if !input.is_predicted() {
				apply_input(character, &input.state, &mut ctx.diff);
			}
		}
	}
}

//can call this in an Immediate or WaitForServer context.
//WaitForConsensus would be janky/unsmooth
fn apply_input(
	character: &mut Character,
	input: &InputState,
	diff: &mut DiffSerializer<impl ImmediateOrWaitForServer>,
) {
	let rot = Quat::from_axis_angle(Vec3::Y, input.cam_yaw);
	character.set_rot(rot, diff);

	let forward = rot * Vec3::NEG_Z;
	let right = forward.cross(Vec3::Y);

	let mut pos = character.get_pos();
	pos += right * input.omnidir.x * SPEED * TickInfo::SIM_DT; //left/right
	pos += Vec3::Y * input.omnidir.y * SPEED * TickInfo::SIM_DT; //up/down
	pos += forward * input.omnidir.z * SPEED * TickInfo::SIM_DT; //forward/backward
	character.set_pos(pos, diff);
}
