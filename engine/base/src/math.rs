use crate::interpolation::Interpolate;
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

pub fn map_range<T>(value: f32, in_min: f32, in_max: f32, out_min: T, out_max: T) -> T
where
	T: Interpolate,
{
	let amount = (value - in_min) / (in_max - in_min);
	T::interpolate(&out_min, &out_max, amount)
}
