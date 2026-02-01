use std::str::FromStr;

use crate::presentation::{camera, get_local_entity};
use base::interpolation::interpolate_entities;
use base::prelude::*;
use js_sys::{JsString, Reflect};
use wasm_bindgen::JsValue;

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
		let entity = get_local_entity(cur_tick, bindings);
		Reflect::set(
			&entity.js.o3d,
			&JsString::from_str("visible").unwrap(),
			&JsValue::FALSE,
		)
		.unwrap();
	}
}
