use borger::interpolation::{InterpolateTicks, InterpolationOutput};
use borger::presentation::{PresentationState, SimulationOutput};
use borger::simulation_controller::{self, SimControllerExternals};
use borger::simulation_state::Input;
use borger::thread_comms::{PresentationToSimCommand, SimToPresentationCommand};
use game_rs::input;
use js_sys::{Function, Uint8Array};
use log::Level;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::{mem, panic};
use wasm_bindgen::prelude::*;
use web_sys::console;
use web_time::Instant;

#[cfg(feature = "session_replay")]
use borger::thread_comms::SessionReplayAction;

#[cfg(not(debug_assertions))]
const LOG_LEVEL: Level = Level::Info;
#[cfg(debug_assertions)]
const LOG_LEVEL: Level = Level::Debug;

#[wasm_bindgen]
pub struct PresentationController {
	sim: SimControllerExternals,
	now: Instant,
	write_input: Function, //(tx: Uint8Array) => void

	//a bit clunky, but these are the fields who
	//must wait on the simulation thread to init
	//before they can be used (hence option type)
	next_tick_i: bool,
	tick_buffers: [Option<SimulationOutput>; 2],

	input: Input,
	output: Option<InterpolationOutput>,

	#[cfg(feature = "session_replay")]
	session_recording: Vec<SessionReplayAction>,
}

#[wasm_bindgen]
impl PresentationController {
	#[wasm_bindgen(constructor)]
	pub fn new(
		new_client_snapshot: Uint8Array,
		#[wasm_bindgen(unchecked_param_type = "(tx: Uint8Array) => void")] write_input: Function,
	) -> Self {
		log_setup();

		let new_client_snapshot = new_client_snapshot.to_vec();

		#[cfg(feature = "session_replay")]
		let session_recording = vec![SessionReplayAction::Init(new_client_snapshot.clone())];

		Self {
			sim: simulation_controller::init(game_rs::init(), new_client_snapshot),
			now: Instant::now(),
			write_input,

			next_tick_i: false,
			tick_buffers: [None, None],

			input: Input::default(),
			output: None,

			#[cfg(feature = "session_replay")]
			session_recording,
		}
	}

	//safety: this should be called by a wasm-bindgen-wrapped
	//PresentationController owned by js memory in order to get
	//a stable pointer
	pub unsafe fn get_input_ptr(&mut self) -> *mut Input {
		&mut self.input as *mut Input
	}

	//presentation loop is slightly behind simulation loop, so
	//tick buffers are older snapshots of the simulation. the
	//unfortunate downside is that there will always be at least
	//1 frame delay before seeing the consequences
	pub fn presentation_tick(&mut self, dt: f32) -> Option<*const InterpolationOutput> {
		//received input state: send to server
		//(these are old input states that have
		//been merged+validated+diff compressed,
		//not the fresh one passed as an argument)
		while let Ok(sim_msg) = self.sim.comms.from_sim.try_recv() {
			match sim_msg {
				SimToPresentationCommand::InputDiff(input_diff) => {
					self.write_input
						.call1(&JsValue::NULL, &Uint8Array::from(input_diff.as_slice()).into())
						.unwrap();
				}

				#[cfg(feature = "session_replay")]
				SimToPresentationCommand::SessionReplayAction(action) => self.session_recording.push(action),
			};
		}

		//receive tick buffer from simulation thread
		let next_tick = self.sim.presentation_receiver.take(Ordering::AcqRel);
		let received_new_tick = next_tick.is_some();

		if received_new_tick {
			//received presentation output: swap buffers
			self.tick_buffers[self.next_tick_i as usize] = Some(*next_tick.unwrap());
			self.next_tick_i = !self.next_tick_i;
		}

		let Some(cur_tick) = self.tick_buffers[(!self.next_tick_i) as usize].as_ref() else {
			return None;
		};

		let prv_tick = self.tick_buffers[(self.next_tick_i) as usize].as_ref();

		let interp_amount = if let Some(prv_tick) = prv_tick {
			let desired_time = self.now + Duration::from_secs_f32(dt);
			self.now = desired_time.clamp(prv_tick.time, cur_tick.time);
			(self.now - prv_tick.time).as_secs_f32() / (cur_tick.time - prv_tick.time).as_secs_f32()
		} else {
			0.0
		};

		//need to store the result in some rust-owned memory to avoid
		//dropping before js is able to borrow it
		self.output = Some(InterpolationOutput {
			local_client_id: cur_tick.local_client_id,
			state: PresentationState::interpolate_and_diff(
				prv_tick.map(|prv| &prv.state),
				&cur_tick.state,
				interp_amount,
				received_new_tick,
			),
		});

		//input state is only meaningful once rendering begins
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::RawInput(mem::take(&mut self.input)))
			.unwrap();

		Some(self.output.as_ref().unwrap() as *const InterpolationOutput)
	}

	pub fn listen_for_state(&self, state: &Uint8Array) {
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::ReceiveState(state.to_vec()))
			.unwrap();
	}

	#[cfg(feature = "session_replay")]
	pub fn dump_session(&self) -> Vec<u8> {
		postcard::to_allocvec(&self.session_recording).unwrap()
	}

	#[cfg(feature = "session_replay")]
	pub fn replay_session(data: Vec<u8>) {
		log_setup();
		simulation_controller::replay_session(game_rs::init(), postcard::from_bytes(&data).unwrap()).unwrap();
	}

	pub fn abort_simulation(&self) {
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::Abort)
			.unwrap();
	}
}

fn log_setup() {
	panic::set_hook(Box::new(|info| {
		console::log_2(
			&JsValue::from("%c".to_string() + &info.to_string()),
			&JsValue::from("background-color: #FCEBEB;"),
		)
	}));

	console_log::init_with_level(LOG_LEVEL).unwrap();
}

//safety: this is meant to be called in js on the presentation
//controller's Borger.Input
#[wasm_bindgen]
pub unsafe fn validate_input(input: *mut Input) {
	let input = unsafe { &mut *input };
	*input = input::validate(input);
}
