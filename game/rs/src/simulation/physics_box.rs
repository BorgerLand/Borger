use crate::simulation::physstep::GROUP_PUSHABLE;
use base::prelude::*;
use rapier3d::prelude::*;

const PHYSICS_BOX_SIZE: f32 = 1.0;

#[cfg(feature = "server")]
pub fn on_server_start(state: &mut SimulationState, diff: &mut DiffSerializer<WaitForConsensus>) {
	for i in 0..250 {
		let phys_box = state.boxes.add(diff).1;
		phys_box.set_pos(Vec3A::new(0.0, 2.0 * (i + 1) as f32, -5.0), diff);
	}
}

pub fn update_pre_physstep(ctx: &mut GameContext<Immediate>) {
	for phys_box in ctx.state.boxes.values_mut() {
		let mut rb = RigidBodyBuilder::dynamic()
			.pose(Pose3 {
				translation: phys_box.get_pos().into(),
				rotation: phys_box.get_rot(),
			})
			.linvel(phys_box.get_linvel().into())
			.angvel(phys_box.get_angvel().into())
			.sleeping(phys_box.get_sleeping())
			.build();

		rb.activation_mut().time_since_can_sleep = phys_box.get_time_since_can_sleep();

		let col = ColliderBuilder::cuboid(
			PHYSICS_BOX_SIZE / 2.0,
			PHYSICS_BOX_SIZE / 2.0,
			PHYSICS_BOX_SIZE / 2.0,
		)
		.collision_groups(InteractionGroups::new(
			GROUP_PUSHABLE, //i am pushable
			Group::ALL,     //i collide with anything
			InteractionTestMode::default(),
		));

		let rb_handle = ctx.state.physics.rigid_bodies.insert(rb);
		phys_box.rb_handle = rb_handle;
		ctx.state
			.physics
			.colliders
			.insert_with_parent(col, rb_handle, &mut ctx.state.physics.rigid_bodies);
	}
}

pub fn update_post_physstep(ctx: &mut GameContext<Immediate>) {
	let diff = &mut ctx.diff;
	for phys_box in ctx.state.boxes.values_mut() {
		let rb = ctx.state.physics.rigid_bodies.get(phys_box.rb_handle).unwrap();
		phys_box
			.set_pos(rb.position().translation.into(), diff)
			.set_rot(rb.position().rotation, diff)
			.set_linvel(rb.vels().linvel.into(), diff)
			.set_angvel(rb.vels().angvel.into(), diff)
			.set_sleeping(rb.activation().sleeping, diff);

		phys_box.set_time_since_can_sleep(rb.activation().time_since_can_sleep, diff);
	}
}
