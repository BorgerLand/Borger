use borger::interpolation::{InterpolateTicks, InterpolationContext};
use borger::presentation::{PresentationContext, PresentationOutput};
use borger::simulation::Input;
use borger::simulation_controller::{self, SimControllerExternals};
use borger::thread_comms::{PresentationToSimCommand, SimToPresentationCommand};
use game::input;
use js_sys::{Function, Uint8Array};
use log::Level;
use std::sync::atomic::{AtomicBool, Ordering};
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
	write_input: Function, //(tx: Uint8Array<ArrayBuffer>) => void

	//a bit clunky, but these are the fields who
	//must wait on the simulation thread to init
	//before they can be used (hence option type)
	next_tick_i: bool,
	tick_buffers: [Option<PresentationContext>; 2],

	input: Input,
	output: Option<InterpolationContext>,

	#[cfg(feature = "session_replay")]
	session_recording: Vec<SessionReplayAction>,
}

#[wasm_bindgen]
impl PresentationController {
	#[wasm_bindgen(constructor)]
	pub fn new(
		new_client_snapshot: Vec<u8>,
		#[wasm_bindgen(unchecked_param_type = "(tx: Uint8Array<ArrayBuffer>) => void")] write_input: Function,
	) -> Self {
		log_setup();

		let new_client_snapshot = new_client_snapshot;

		#[cfg(feature = "session_replay")]
		let session_recording = vec![SessionReplayAction::Init(new_client_snapshot.clone())];

		Self {
			#[cfg(not(feature = "singlethreaded"))]
			sim: simulation_controller::init_multithreaded(game::init(), new_client_snapshot),
			#[cfg(feature = "singlethreaded")]
			sim: simulation_controller::init_singlethreaded(game::init(), new_client_snapshot),

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

	//when resuming it is very important to not drop the existing
	//tick buffers, in order to interpolate and dispatch all events
	//that have occurred since disconnecting. this effectively is
	//the same as the presentation loop skipping over many
	//simulation ticks
	pub fn resume_disconnected_session(
		old: Self,
		new_client_snapshot: Vec<u8>,
		#[wasm_bindgen(unchecked_param_type = "(tx: Uint8Array<ArrayBuffer>) => void")] write_input: Function,
	) -> Self {
		let mut new = PresentationController::new(new_client_snapshot, write_input);
		new.next_tick_i = old.next_tick_i;
		new.tick_buffers = old.tick_buffers;
		new
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
	//1 frame delay before seeing the consequences. this will
	//not return some until simulation thread has produced its
	//first tick
	pub fn presentation_tick(&mut self, dt: f32) -> Option<*const InterpolationContext> {
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

		#[cfg(feature = "singlethreaded")]
		self.sim.loop_singlethreaded();

		//receive tick buffer from simulation thread
		let next_tick = self.sim.comms.sim_out.take(Ordering::AcqRel);
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
			self.now = cur_tick.time;
			0.0
		};

		//need to store the result in some rust-owned memory to avoid
		//dropping before js is able to borrow it
		self.output = Some(InterpolationContext {
			local_client_id: cur_tick.local_client_id,
			output: PresentationOutput::interpolate_and_diff(
				prv_tick.map(|prv| &prv.output),
				&cur_tick.output,
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

		Some(self.output.as_ref().unwrap() as *const InterpolationContext)
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
		simulation_controller::replay_session(game::init(), postcard::from_bytes(&data).unwrap()).unwrap();
	}
}

static LOG_SETUP_COMPLETE: AtomicBool = AtomicBool::new(false);
fn log_setup() {
	if LOG_SETUP_COMPLETE.swap(true, Ordering::Relaxed) {
		return;
	}

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
