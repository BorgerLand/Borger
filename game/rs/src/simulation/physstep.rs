use base::prelude::*;
use glam::Vec3;

pub const GRAVITY: Vec3 = Vec3::new(0.0, -30.0, 0.0);

pub fn update(ctx: &mut GameContext<Immediate>) {
	let mut start_physics_test = false;
	for client in ctx.state.clients.values() {
		if let ClientState::Owned(client) = client {
			if client.input.get().start_physics_test {
				start_physics_test = true;
				break;
			}
		}
	}

	if start_physics_test {
		ctx.state.set_running_physics_test(true, &mut ctx.diff);
		for (_, rb) in ctx.state.physics.rigid_bodies.iter_mut() {
			rb.wake_up(true);
		}
	}

	let gravity = if ctx.state.get_running_physics_test() {
		GRAVITY
	} else {
		Vec3::ZERO
	};

	ctx.state.physics.step(gravity);
}
