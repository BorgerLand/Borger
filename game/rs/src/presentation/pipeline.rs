use crate::presentation::camera;
use base::{entities, prelude::*};

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
	entities::interpolate(
		prv_tick,
		cur_tick,
		received_new_tick,
		interpolation_amount,
		bindings,
	);

	camera::update(cur_tick, input, bindings);
}
