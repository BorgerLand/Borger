use base::math::wrap_angle;
use base::networked_types::primitive::usize32;
use base::simulation_state::InputState;
use base::simulation_state::SimulationState;
use glam::Vec2;

#[cfg(feature = "client")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "client")]
#[wasm_bindgen]
pub fn populate_input(
	input: &mut InputState,
	pointer_dx: f32,
	pointer_dy: f32,
	omnidir_x: f32,
	omnidir_y: f32,
	jumping: bool,
	start_physics_test: bool,
	blow_nose: bool,
) {
	*input = InputState {
		cam_yaw: input.cam_yaw - pointer_dx,
		cam_pitch: input.cam_pitch + pointer_dy,
		cam_radius: 0.0,

		omnidir: Vec2::new(omnidir_x, omnidir_y),
		jumping,

		start_physics_test,
		blow_nose,
	};

	validate(input);
}

//a single client has multiple input states per simulation
//tick due to vsync outpacing the simulation tick rate.
//this function merges them down into one
#[cfg(feature = "client")]
pub fn merge(combined: &mut InputState, new: &InputState) {
	*combined = InputState {
		//camera persists between frames, so always take the newest
		cam_yaw: new.cam_yaw,
		cam_pitch: new.cam_pitch,
		cam_radius: new.cam_radius,

		//take newest nipple/omnidir if it exists. if not, don't overwrite the old one.
		//allows very short sub-1-tick nipple movements to go through
		omnidir: if new.omnidir != Vec2::ZERO {
			new.omnidir
		} else {
			combined.omnidir
		},
		jumping: combined.jumping || new.jumping,

		start_physics_test: combined.start_physics_test || new.start_physics_test,
		blow_nose: combined.blow_nose || new.blow_nose,
	};
}

//given a suspicious, untrustworthy input state,
//return a new sanitized version
pub fn validate(sus: &mut InputState) {
	//be sure to pass all floating point (decimal) numbers
	//through valid_fXX(). otherwise you have a security
	//problem where an evil client can blow up the game.
	//any math equation that receives an infinity/nan
	//value will return even more infinity/nan values, and
	//the whole game state is taken down like a jessie j
	//domino

	//this should only validate that the one isolated
	//input state it receives makes sense. checking for
	//eg. debounce or other timings between multiple
	//input state objects is out of scope

	*sus = InputState {
		cam_yaw: wrap_angle(valid_f32(sus.cam_yaw)),
		cam_pitch: valid_f32(sus.cam_pitch).clamp(-89.9_f32.to_radians(), 89.9_f32.to_radians()),
		cam_radius: valid_f32(sus.cam_radius).clamp(0., f32::INFINITY),

		omnidir: Vec2::new(
			valid_f32(sus.omnidir.x).clamp(-1., 1.),
			valid_f32(sus.omnidir.y).clamp(-1., 1.),
		)
		.normalize_or_zero(),
		jumping: sus.jumping,

		start_physics_test: sus.start_physics_test,
		blow_nose: sus.blow_nose,
	};
}

//the server needs to continue simulating even if it hasn't
//received inputs from all clients yet, and a client needs
//to continue simulating even if it hasn't received a new
//input from the presentation thread yet. this function lets
//you choose how the engine fabricates an input, given the
//previous tick's input. if accessing state.clients[client_id]:
//the clientstate will always be owned. do not access
//client.input; it will be wrong; use last_known instead
pub fn predict_late(last_known: &InputState, _state: &SimulationState, _client_id: usize32) -> InputState {
	//default to assuming the client stopped pressing
	//anything, and the camera hasn't moved. in racing/
	//vehicle games it's more common to assume they
	//will continue pressing the throttle, but trying
	//this on running+jumping character puts them at
	//risk of running off cliffs. push-and-hold buttons
	//(eg. left click, controller triggers) are also
	//usually safe to assume they are still in the same
	//position
	InputState {
		cam_yaw: last_known.cam_yaw,
		cam_pitch: last_known.cam_pitch,
		cam_radius: last_known.cam_radius,

		omnidir: Default::default(),
		jumping: false,

		start_physics_test: false,
		blow_nose: false,
	}
}

#[allow(dead_code)]
fn valid_f32(sus: f32) -> f32 {
	if sus.is_finite() { sus } else { 0.0 }
}

#[allow(dead_code)]
fn valid_f64(sus: f64) -> f64 {
	if sus.is_finite() { sus } else { 0.0 }
}
