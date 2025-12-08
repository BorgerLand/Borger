use crate::presentation::camera;
use base::prelude::*;

//see comments in Pipeline.ts: this allows calling
//rust code during the js-driven presentation loop
pub fn presentation_tick(
	_prv_tick: Option<&PresentationTick>,
	cur_tick: &PresentationTick,
	_received_new_tick: bool,
	_interpolation_amount: f32,
	input: &InputState,
	bindings: &mut JSBindings,
) {
	camera::update(cur_tick, input, bindings);
}
