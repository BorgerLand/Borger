use crate::simulation::physstep::{GRAVITY, GROUP_CHARACTER, GROUP_PUSHABLE};
use base::networked_types::collections::slotmap::SlotMap;
use base::prelude::*;
use glam::{EulerRot, Quat, Vec3, Vec3A};
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::parry::shape::Capsule;
use rapier3d::prelude::*;

#[cfg(feature = "server")]
use base::networked_types::primitive::usize32;

const RADIUS: f32 = 0.35;
const HEIGHT: f32 = 2.2;
const EYE_HEIGHT: f32 = 2.55;
const CONTROLLER_OFFSET: f32 = 0.01;
const KINEMATIC_OFFSET: f32 = 0.3; //affects how hard characters have to push on pushables
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
	client.set_id(character.0, diff);

	let spawn_pos = to_center_pos(Vec3::ZERO).into();
	character.1.set_prv_pos(spawn_pos, diff).set_pos(spawn_pos, diff);
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

//update kinematic: a rigid body attached to the character for the purpose
//of controllers colliding with one another + rigid bodies are pushed out
//of the way
pub fn update_pre_physstep(ctx: &mut GameContext<impl ImmediateOrWaitForServer>) {
	let kinematic_shape = SharedShape::new(Capsule::new_y(
		half_cyl() - KINEMATIC_OFFSET,
		RADIUS + KINEMATIC_OFFSET,
	));

	//remember: the server "owns" all client objects.
	//a locally running client only owns their own client
	//object. the "input" field has owner visibility,
	//so effectively the server simulates all players
	//while each client only simulates their own. the
	//server then informs all players of where all the
	//"remote" players are
	for client in ctx.state.clients.owned_clients_mut() {
		//place a kinematic body at the previous position and
		//move it to the current position to shove objects in
		//between the 2 positions
		let character = ctx.state.characters.get_mut(client.get_id()).unwrap();

		let rb = RigidBodyBuilder::kinematic_position_based().pose(Pose3 {
			translation: character.get_prv_pos().into(),
			rotation: Quat::IDENTITY,
		});

		let col = ColliderBuilder::new(kinematic_shape.clone()).collision_groups(InteractionGroups::new(
			GROUP_CHARACTER, //i am a character
			GROUP_PUSHABLE,  //i collide with pushables
			InteractionTestMode::default(),
		));

		let rb_handle = ctx.state.physics.rigid_bodies.insert(rb);
		ctx.state
			.physics
			.colliders
			.insert_with_parent(col, rb_handle, &mut ctx.state.physics.rigid_bodies);

		ctx.state
			.physics
			.rigid_bodies
			.get_mut(rb_handle)
			.unwrap()
			.set_next_kinematic_translation(character.get_pos().into());

		character.set_prv_pos(character.get_pos(), &mut ctx.diff);
	}
}

//update controller: actually moves the character given a desired translation
pub fn update_post_physstep(ctx: &mut GameContext<impl ImmediateOrWaitForServer>) {
	let mut controller = KinematicCharacterController::default();
	controller.offset = CharacterLength::Absolute(CONTROLLER_OFFSET);
	let controller_shape = Capsule::new_y(half_cyl(), RADIUS - CONTROLLER_OFFSET);
	let diff = &mut ctx.diff;

	for client in ctx.state.clients.owned_clients_mut() {
		let character = ctx.state.characters.get_mut(client.get_id()).unwrap();
		let input = client.input.get();

		//rotation has no effect on physics/game logic but is used
		//for camera+mesh rotation
		let rot = Quat::from_axis_angle(Vec3::Y, input.cam_yaw);
		character.set_rot(rot, diff);

		let phys = &ctx.state.physics;
		let (translation, mut vel) = get_desired_movement(character, input);
		let mut center_pos = character.get_pos().into();

		let result = controller.move_shape(
			TickInfo::SIM_DT,
			&phys.broad_phase.as_query_pipeline(
				phys.narrow_phase.query_dispatcher(),
				&phys.rigid_bodies,
				&phys.colliders,
				QueryFilter::default().groups(InteractionGroups::new(
					GROUP_CHARACTER,              //i am a character
					Group::ALL ^ GROUP_CHARACTER, //i collide with anything except characters
					InteractionTestMode::default(),
				)),
			),
			&controller_shape,
			&Pose3 {
				translation: center_pos,
				rotation: Quat::IDENTITY,
			},
			translation.into(),
			|_| {},
		);

		center_pos += result.translation;
		if result.grounded {
			vel = Vec3A::ZERO;
		} else {
			vel += Vec3A::from(GRAVITY) * TickInfo::SIM_DT;
			vel = vel.clamp_length_max(TERMINAL_VELOCITY);
		}

		character
			.set_velocity(vel, diff)
			.set_pos(center_pos.into(), diff)
			.set_grounded(result.grounded, diff);
	}
}

//-> (translation, velocity)
fn get_desired_movement(character: &Character, input: &InputState) -> (Vec3, Vec3A) {
	const UP: Vec3A = Vec3A::Y;
	let forward = character.get_rot() * Vec3A::NEG_Z;
	let right = forward.cross(UP);

	let vel = if character.get_grounded() && input.jumping {
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

	(desired.into(), vel)
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

const fn half_cyl() -> f32 {
	HEIGHT / 2.0 - RADIUS
}

pub fn to_center_pos(mut foot_pos: Vec3) -> Vec3 {
	foot_pos.y += RADIUS + half_cyl();
	foot_pos
}

pub fn to_eye_pos(mut center_pos: Vec3) -> Vec3 {
	center_pos.y += EYE_HEIGHT - (RADIUS + half_cyl());
	center_pos
}
