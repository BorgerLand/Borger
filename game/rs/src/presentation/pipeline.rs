use crate::presentation::{camera, get_local_entity};
use base::interpolation::interpolate_entities;
use base::prelude::*;
use js_sys::{Function, Reflect};

//see comments in Pipeline.ts: this allows calling
//rust code during the js-driven presentation loop
pub fn presentation_tick(
	prv_tick: Option<&SimulationOutput>,
	cur_tick: &SimulationOutput,
	received_new_tick: bool,
	interpolation_amount: f32,
	input: &InputState,
	bindings: &mut JSBindings,
) {
	interpolate_entities(
		prv_tick,
		cur_tick,
		received_new_tick,
		interpolation_amount,
		bindings,
	);

	camera::update(cur_tick, input, bindings);

	//make your own character invisible upon startup
	if prv_tick.is_none() {
		let o3d = &get_local_entity(cur_tick, bindings).js.o3d;
		Function::from(Reflect::get(o3d, &bindings.cache.remove_from_parent_str).unwrap())
			.call0(o3d)
			.unwrap();
	}
}
