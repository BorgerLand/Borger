use crate::simulation::physstep::GRAVITY;
use base::networked_types::collections::slotmap::SlotMap;
use base::prelude::*;
use glam::{EulerRot, Quat, Vec3, Vec3A};
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::parry::shape::Capsule;
use rapier3d::prelude::*;

#[cfg(feature = "server")]
use base::networked_types::primitive::usize32;

const RADIUS: f32 = 0.35;
const CYL_HEIGHT: f32 = 2.2;
const EYE_HEIGHT: f32 = 2.55;
const OFFSET: f32 = 0.01;
const SPEED: f32 = 9.0; //units/sec
const TERMINAL_VELOCITY: f32 = 60.0; //units/sec
const JUMP_VELOCITY: f32 = 11.0; //units/sec

#[cfg(feature = "server")]
pub fn on_client_connect(
	state: &mut SimulationState,
	client_id: usize32,
	diff: &mut DiffSerializer<WaitForConsensus>,
) {
	let client = get_owned_client_mut(&mut state.clients, client_id).unwrap();
	let character = state.characters.add(diff);
	character.1.set_pos(to_center_pos(Vec3::ZERO).into(), diff);
	client.set_id(character.0, diff);
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

struct ControllerQueryData<'a> {
	controller: KinematicCharacterController,
	shape: Capsule,
	pipeline: QueryPipeline<'a>,
}

pub fn update(ctx: &mut GameContext<Immediate>) {
	let mut controller = KinematicCharacterController::default();
	controller.offset = CharacterLength::Absolute(OFFSET);

	let phys = &ctx.state.physics;
	let query_pipeline = phys.broad_phase.as_query_pipeline(
		phys.narrow_phase.query_dispatcher(),
		&phys.rigid_bodies,
		&phys.colliders,
		QueryFilter::default(),
	);

	let query_data = ControllerQueryData {
		controller,
		shape: Capsule::new_y(CYL_HEIGHT / 2.0, RADIUS - OFFSET),
		pipeline: query_pipeline,
	};

	//remember: the server "owns" all client objects.
	//a locally running client only owns their own client
	//object. the "input" field has owner visibility,
	//so effectively the server simulates all players
	//while each client only simulates their own. the
	//server then informs all players of where all the
	//"remote" players are
	for client in ctx.state.clients.owned_clients_mut() {
		let character = ctx.state.characters.get_mut(client.get_id()).unwrap();
		apply_input(
			character,
			client.input.get(),
			&query_data,
			&ctx.tick,
			&mut ctx.diff,
		);
	}
}

//can call this in an Immediate or WaitForServer context.
//WaitForConsensus would be janky/unsmooth
fn apply_input(
	character: &mut Character,
	input: &InputState,
	query_data: &ControllerQueryData,
	tick: &TickInfo,
	diff: &mut DiffSerializer<impl ImmediateOrWaitForServer>,
) {
	let rot = Quat::from_axis_angle(Vec3::Y, input.cam_yaw);
	character.set_rot(rot, diff);

	const UP: Vec3A = Vec3A::Y;
	let forward = rot * Vec3A::NEG_Z;
	let right = forward.cross(UP);

	let mut vel = if character.get_grounded() && input.jumping {
		Vec3A::new(0.0, JUMP_VELOCITY, 0.0)
	} else {
		character.get_velocity()
	};

	let mut desired = Vec3A::ZERO;
	desired += right * input.omnidir.x; //left/right
	//desired += UP * input.omnidir.y; //up/down
	desired += forward * input.omnidir.y; //forward/backward
	desired *= SPEED;
	desired += vel + 0.5 * Vec3A::from(GRAVITY) * TickInfo::SIM_DT;
	desired *= TickInfo::SIM_DT;

	let center_pos = character.get_pos().into();
	let result = query_data.controller.move_shape(
		TickInfo::SIM_DT,
		&query_data.pipeline,
		&query_data.shape,
		&Pose3 {
			translation: center_pos,
			rotation: Quat::IDENTITY,
		},
		desired.into(),
		|_| {},
	);

	multiplayer_tradeoff!(
		WaitForConsensus,
		diff,
		tick,
		println!(
			"new pos: {:?} grounded: {} desired: {:?} actual: {:?}",
			center_pos + result.translation,
			result.grounded,
			desired,
			result.translation
		)
	);
	character.set_grounded(result.grounded, diff);
	if result.grounded {
		vel = Vec3A::ZERO;
	} else {
		vel += Vec3A::from(GRAVITY) * TickInfo::SIM_DT;
		vel = vel.clamp_length_max(TERMINAL_VELOCITY);
	}

	character
		.set_velocity(vel, diff)
		.set_pos((center_pos + result.translation).into(), diff);
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

pub fn to_center_pos(mut foot_pos: Vec3) -> Vec3 {
	foot_pos.y += RADIUS + CYL_HEIGHT / 2.0;
	foot_pos
}

pub fn to_eye_pos(mut center_pos: Vec3) -> Vec3 {
	center_pos.y += EYE_HEIGHT - (RADIUS + CYL_HEIGHT / 2.0);
	center_pos
}
