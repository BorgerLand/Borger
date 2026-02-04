use base::js_bindings::{JSBindings, bind_camera};
use base::networked_types::primitive::usize_to_32;
use base::presentation_state::SimulationOutput;
use base::simulation_controller::SimControllerExternals;
use base::simulation_state::InputState;
use base::thread_comms::{PresentationToSimCommand, SimToPresentationCommand};
use game_rs::presentation::on_client_start;
use game_rs::presentation::pipeline::presentation_tick as game_presentation_tick;
use js_sys::{Function, Uint8Array};
use std::panic;
use std::sync::atomic::Ordering;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use web_sys::{WritableStreamDefaultWriter, console};
use web_time::Instant;

#[wasm_bindgen]
pub struct PresentationController {
	sim: SimControllerExternals,
	now: Instant,
	bindings: JSBindings,

	//a bit clunky, but these are the fields who
	//must wait on the simulation thread to init
	//before they can be used (hence option type)
	pub is_ready: bool,
	tick_buffers: [Option<SimulationOutput>; 2],
	next_tick_i: bool,
}

#[wasm_bindgen]
impl PresentationController {
	#[wasm_bindgen(constructor)]
	pub fn new(
		new_client_snapshot: Uint8Array,
		input_stream: WritableStreamDefaultWriter,

		#[wasm_bindgen(unchecked_param_type = "import('three').Scene")] scene: &JsValue,
		#[wasm_bindgen(unchecked_param_type = "(type: EntityType, id: number) => import('three').Object3D")]
		spawn_entity_cb: Function,
		#[wasm_bindgen(
			unchecked_param_type = "(type: EntityType, entity: import('three').Object3D, id: number) => void"
		)]
		dispose_entity_cb: Function,
	) -> Self {
		panic::set_hook(Box::new(|info| {
			console::log_2(
				&JsValue::from("%c".to_string() + &info.to_string()),
				&JsValue::from("background-color: #FCEBEB;"),
			)
		}));

		console_log::init().unwrap();

		Self {
			sim: game_rs::simulation::init(new_client_snapshot.to_vec()),
			now: Instant::now(),
			bindings: JSBindings::new(input_stream, scene, spawn_entity_cb, dispose_entity_cb),

			is_ready: false,
			tick_buffers: [None, None],
			next_tick_i: false,
		}
	}

	//must be called from js, not in constructor, due to safety requirements
	pub fn init_pinned(
		&mut self,

		#[wasm_bindgen(unchecked_param_type = "import('three').Camera")] camera: &JsValue,
	) {
		unsafe {
			bind_camera(&self.bindings.camera, camera, &self.bindings.cache);
		}

		on_client_start(&mut self.bindings);
	}

	//presentation loop is slightly behind simulation loop, so tick buffers are older snapshots of the simulation
	//the unfortunate downside is that there will always be at least 1 frame delay before seeing the consequences
	pub fn presentation_tick(&mut self, dt: f32, input: &mut InputState) {
		//received input state: send to server
		//(these are old input states that have
		//been merged+validated+diff compressed,
		//not the fresh one passed as an argument)
		while let Ok(sim_msg) = self.sim.comms.from_sim.try_recv() {
			match sim_msg {
				SimToPresentationCommand::InputDiff(input_diff) => {
					#[allow(unused_must_use)]
					self.bindings.cache.input_stream.write_with_chunk(
						&Uint8Array::from(usize_to_32(input_diff.len()).to_le_bytes().as_slice()).into(),
					);
					#[allow(unused_must_use)]
					self.bindings
						.cache
						.input_stream
						.write_with_chunk(&Uint8Array::from(input_diff.as_slice()).into());
				}
			};
		}

		//receive tick buffer from simulation thread
		let nxt_tick = self.sim.presentation_receiver.take(Ordering::AcqRel);
		let received_tick = nxt_tick.is_some();

		if received_tick {
			//received presentation output: swap buffers
			self.tick_buffers[self.next_tick_i as usize] = Some(*nxt_tick.unwrap());
			self.next_tick_i = !self.next_tick_i;
		}

		let prv_tick = self.tick_buffers[(self.next_tick_i) as usize].as_ref();
		let cur_tick = self.tick_buffers[(!self.next_tick_i) as usize].as_ref();

		if !self.is_ready {
			//if not previously ready, check if ready now
			self.is_ready = prv_tick.is_some() && cur_tick.is_some();

			//no need to do full interpolation if not rendering yet,
			//but still need to call the game's presentation tick in
			//order to fire binding/spawning events
			if received_tick {
				game_presentation_tick(prv_tick, cur_tick.unwrap(), true, 0., input, &mut self.bindings);
			}

			//if is_ready == true at this point, the next
			//tick will be the first rendered frame
			return;
		}

		//input state is only meaningful once rendering begins
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::RawInput(input.clone()))
			.unwrap();

		let prv_tick_uw = prv_tick.unwrap();
		let cur_tick = cur_tick.unwrap();

		let desired_time = self.now + Duration::from_secs_f32(dt);
		self.now = desired_time.clamp(prv_tick_uw.time, cur_tick.time);
		let interp_amount =
			(self.now - prv_tick_uw.time).as_secs_f32() / (cur_tick.time - prv_tick_uw.time).as_secs_f32();

		game_presentation_tick(
			prv_tick,
			cur_tick,
			received_tick,
			interp_amount,
			input,
			&mut self.bindings,
		);
	}

	pub fn listen_for_state(&self, state: &Uint8Array) {
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::ReceiveState(state.to_vec()))
			.unwrap();
	}

	pub fn abort_simulation(&self) {
		self.sim
			.comms
			.to_sim
			.send(PresentationToSimCommand::Abort)
			.unwrap();
	}
}
