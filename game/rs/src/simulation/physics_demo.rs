use crate::simulation::physstep::GROUP_PUSHABLE;
use base::prelude::*;
use rapier3d::prelude::*;

const PHYSICS_CUBE_SIZE: f32 = 1.0;
const PHYSICS_SPHERE_RADIUS: f32 = 0.5;

#[cfg(feature = "server")]
pub fn on_server_start(state: &mut SimulationState, diff: &mut DiffSerializer<WaitForConsensus>) {
	for i in 0..250 {
		let cube = state.cubes.add(diff).1;
		cube.set_pos(Vec3A::new(0.0, 2.0 * (i + 1) as f32, -5.0), diff);
	}
}

//insert the rigid body into the physics engine
macro_rules! update_pre_physstep {
	($ctx:expr, $entities:expr, $make_collider:expr) => {
		for entity in $entities.values_mut() {
			let mut rb = RigidBodyBuilder::dynamic()
				.pose(Pose3 {
					translation: entity.get_pos().into(),
					rotation: entity.get_rot(),
				})
				.linvel(entity.get_linvel().into())
				.angvel(entity.get_angvel().into())
				.sleeping(entity.get_sleeping())
				.build();

			rb.activation_mut().time_since_can_sleep = entity.get_time_since_can_sleep();

			let rb_handle = $ctx.state.physics.rigid_bodies.insert(rb);
			entity.rb_handle = rb_handle;
			$ctx.state.physics.colliders.insert_with_parent(
				$make_collider,
				rb_handle,
				&mut $ctx.state.physics.rigid_bodies,
			);
		}
	};
}

//after stepping the simulation, retrieve and store the
//rigid body's state
macro_rules! update_post_physstep {
	($ctx:expr, $entities:expr) => {{
		let diff = &mut $ctx.diff;
		let mut despawn = Vec::new();

		for (id, entity) in $entities.iter_mut() {
			let rb = $ctx.state.physics.rigid_bodies.get(entity.rb_handle).unwrap();
			if rb.position().translation.y > -10.0 {
				entity
					.set_pos(rb.position().translation.into(), diff)
					.set_rot(rb.position().rotation, diff)
					.set_linvel(rb.vels().linvel.into(), diff)
					.set_angvel(rb.vels().angvel.into(), diff)
					.set_sleeping(rb.activation().sleeping, diff);

				entity.set_time_since_can_sleep(rb.activation().time_since_can_sleep, diff);
			} else {
				despawn.push(id);
			}
		}

		for id in despawn {
			$entities.remove(id, diff).unwrap();
		}
	}};
}

fn make_cube_col() -> impl Into<Collider> {
	make_pushable(ColliderBuilder::cuboid(
		PHYSICS_CUBE_SIZE / 2.0,
		PHYSICS_CUBE_SIZE / 2.0,
		PHYSICS_CUBE_SIZE / 2.0,
	))
}

fn make_sphere_col() -> impl Into<Collider> {
	make_pushable(ColliderBuilder::ball(PHYSICS_SPHERE_RADIUS))
}

fn make_pushable(builder: ColliderBuilder) -> ColliderBuilder {
	builder.collision_groups(InteractionGroups::new(
		GROUP_PUSHABLE, //i am pushable
		Group::ALL,     //i collide with anything
		InteractionTestMode::default(),
	))
}

pub fn update_pre_physstep(ctx: &mut GameContext<Immediate>) {
	let diff = &mut ctx.diff;
	for client in ctx.state.clients.owned_clients() {
		if client.input.get().blow_nose {
			ctx.state
				.spheres
				.add(diff)
				.1
				.set_pos(Vec3A::new(-4.2, 16.0, -1.0), diff)
				.set_linvel(Vec3A::new(0.0, 0.0, 10.0), diff);
			ctx.state
				.spheres
				.add(diff)
				.1
				.set_pos(Vec3A::new(4.2, 16.0, -1.0), diff)
				.set_linvel(Vec3A::new(0.0, 0.0, 10.0), diff);
		}
	}

	update_pre_physstep!(ctx, ctx.state.cubes, make_cube_col());
	update_pre_physstep!(ctx, ctx.state.spheres, make_sphere_col());
}

pub fn update_post_physstep(ctx: &mut GameContext<Immediate>) {
	update_post_physstep!(ctx, ctx.state.cubes);
	update_post_physstep!(ctx, ctx.state.spheres);
}
