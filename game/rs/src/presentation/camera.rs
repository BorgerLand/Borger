use crate::presentation::get_local_entity;
use crate::simulation::character;
use base::prelude::*;
use glam::Mat4;

pub fn update(tick: &SimulationOutput, input: &InputState, bindings: &mut JSBindings) {
	//treasure hunt for the position of this client's character
	let cam_target = get_local_entity(tick, bindings).pos;

	//players' toleration for latency between moving the mouse and seeing camera
	//movement is so extremely low that not even multiplayer_tradeoff!(Immediate)
	//is fast enough due to the rtt of the presentation thread sending the camera
	//input, receiving a response, and interpolating towards it. so, cheat here by
	//by directly writing the latest camera input to its rotation. this causes the
	//entity's angle to lag behind the camera angle, but you can't see your own
	//entity in 1st person anyway so it doesn't matter. (it is also possible to
	//just recalculate the entity's matrix here as a hack workaround)

	let cam_rot = character::get_camera_rot(input);
	bindings.camera.mat = Mat4::from_rotation_translation(cam_rot, cam_target.into());
	bindings.camera.mat_inv = bindings.camera.mat.inverse();
}
