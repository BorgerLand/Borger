use base::networked_types::collections::slotmap::SlotMap;
use base::prelude::*;
use glam::{EulerRot, Quat, Vec3, Vec3A};

#[cfg(feature = "server")]
use base::networked_types::primitive::usize32;

const SPEED: f32 = 6.0; //units/sec

#[cfg(feature = "server")]
pub fn on_client_connect(
	state: &mut SimulationState,
	client_id: usize32,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	let client = get_owned_client_mut(&mut state.clients, client_id).unwrap();
	let character = state.characters.add(diff).0;
	client.set_id(character, diff);
}

#[cfg(feature = "server")]
pub fn on_client_disconnect(
	state: &mut SimulationState,
	client_id: usize32,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	let client = get_owned_client(&mut state.clients, client_id).unwrap();
	let character = client.get_id();
	state.characters.remove(character, diff).unwrap();
}

pub fn update(ctx: &mut GameContext<Immediate>) {
	//remember: the server "owns" all client objects.
	//a locally running client only owns their own client
	//object. the "input" field has owner visibility,
	//so effectively the server simulates all players
	//while each client only simulates their own. the
	//server then informs all players of where all the
	//"remote" players are
	for client in ctx.state.clients.owned_clients_mut() {
		let character = ctx.state.characters.get_mut(client.get_id()).unwrap();
		apply_input(character, client.input.get(), &mut ctx.diff);
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

	let forward = rot * Vec3A::NEG_Z;
	let right = forward.cross(Vec3A::Y);

	let mut pos = character.get_pos();
	pos += right * input.omnidir.x * SPEED * TickInfo::SIM_DT; //left/right
	pos += Vec3A::Y * input.omnidir.y * SPEED * TickInfo::SIM_DT; //up/down
	pos += forward * input.omnidir.z * SPEED * TickInfo::SIM_DT; //forward/backward
	character.set_pos(pos, diff);
}

pub fn get_camera_rot(input: &InputState) -> Quat {
	Quat::from_euler(EulerRot::ZYX, 0., input.cam_yaw, input.cam_pitch)
}

pub fn get_character<'a>(client: &ClientState_owned, characters: &'a SlotMap<Character>) -> &'a Character {
	characters.get(client.get_id()).unwrap()
}

pub fn get_character_mut<'a>(
	client: &ClientState_owned,
	characters: &'a mut SlotMap<Character>,
) -> &'a Character {
	characters.get_mut(client.get_id()).unwrap()
}
