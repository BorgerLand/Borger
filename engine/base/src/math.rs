use std::f32::consts::{PI, TAU};

///Shortest distance between two angles in range [-PI, PI)
pub fn angle_distance(cur: f32, prv: f32) -> f32 {
	wrap_angle(cur - prv)
}

///Wrap angle in range [-PI, PI)
pub fn wrap_angle(angle: f32) -> f32 {
	let mut diff = ((angle + PI) % TAU) - PI;
	if diff < -PI {
		diff += TAU;
	}

	diff
}
