use crate::ClientStateKind;
use crate::SimulationCallbacks;
use crate::constructors::ConstructCustomStruct;
use crate::context::{GameContext, Impl};
use crate::diff_ser::DiffSerializer;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::{InputState, SimulationState};
use crate::thread_comms::*;
use crate::tick::{TickID, TickInfo};
use log::debug;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc as sync_mpsc;
use wasm_thread as thread;
use web_time::Instant;

#[cfg(feature = "server")]
use {
	crate::tick::UnrollbackableNetEvent,
	std::collections::HashMap,
	std::sync::mpsc::{Receiver as SyncReceiver, Sender as SyncSender},
	tokio::sync::mpsc::UnboundedSender as AsyncSender,
};

#[cfg(feature = "client")]
use {
	crate::presentation_state::PresentationTick, crate::snapshot_serdes, atomicbox::AtomicOptionBox,
	std::sync::Arc,
};

#[cfg(feature = "client")]
mod client;
mod seek;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "server")]
const TRACE_TICK_ADVANCEMENT: bool = false;
#[cfg(feature = "client")]
const TRACE_TICK_ADVANCEMENT: bool = false;

//communications between the simulation thread
//and the owning parent thread
pub struct SimControllerExternals {
	#[cfg(feature = "server")]
	pub new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "client")]
	pub comms: PresentationToSimChannel,
	#[cfg(feature = "client")]
	pub presentation_receiver: Arc<AtomicOptionBox<PresentationTick>>,
}

//data that needs to be moved across threads
//during initialization of simulation thread.
//all fields must be Send
struct SimulationMoveAcrossThreads {
	cb: SimulationCallbacks,

	#[cfg(feature = "client")]
	new_client_snapshot: VecDeque<u8>,

	#[cfg(feature = "server")]
	new_connection_receiver: SyncReceiver<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "client")]
	comms: SimToPresentationChannel,
	#[cfg(feature = "client")]
	presentation_sender: Arc<AtomicOptionBox<PresentationTick>>,
}

//on client, owned by simulation thread
pub(crate) struct SimControllerInternals {
	pub ctx: GameContext<Impl>,
	cb: SimulationCallbacks,

	//input_history and state_diff both need to keep at least
	//(1 + tick.id_cur - tick.id_consensus) ticks worth of history.
	//it is guaranteed to have this amount of history for at least one
	//client, as well as at least 1 per client, but may be missing some
	//from other clients due to latency (inputs arriving later than
	//others)

	//storage for reconciliation: resimulate inputs when more arrive.
	//server is aware of all clients' inputs; client only stores its own
	#[cfg(feature = "server")]
	input_history: HashMap<usize32, InternalInputHistory>,
	#[cfg(feature = "client")]
	input_history: InternalInputHistory,

	#[cfg(feature = "server")]
	new_connection_receiver: SyncReceiver<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "server")]
	comms: HashMap<usize32, SimToClientChannel>,
	#[cfg(feature = "client")]
	comms: SimToPresentationChannel,
	#[cfg(feature = "client")]
	pub output_sender: Arc<AtomicOptionBox<PresentationTick>>,

	#[cfg(feature = "server")]
	net_events: VecDeque<UnrollbackableNetEvent>,

	#[cfg(feature = "client")]
	pub local_client_id: usize32,
}

#[derive(Default)]
struct InternalInputHistory {
	ticks: VecDeque<InputState>,

	#[cfg(feature = "server")]
	latest_receied: InputState,

	//how many ticks timed out and reached consensus
	//while waiting for this client's inputs. used to
	//prevent newly received inputs from adding to the
	//ticks buffer until there are no more missing
	#[cfg(feature = "server")]
	missing: u32,
}

pub fn init(
	cb: SimulationCallbacks,

	#[cfg(feature = "client")] new_client_snapshot: VecDeque<u8>,
) -> SimControllerExternals {
	#[cfg(feature = "server")]
	let (new_connection_sender, new_connection_receiver) = sync_mpsc::channel();

	#[cfg(feature = "client")]
	let (to_presentation, from_sim) = sync_mpsc::channel();
	#[cfg(feature = "client")]
	let (to_sim, from_presentation) = sync_mpsc::channel();

	#[cfg(feature = "client")]
	let presentation_comms = SimToPresentationChannel {
		to_presentation,
		from_presentation,
	};
	#[cfg(feature = "client")]
	let sim_comms = PresentationToSimChannel { to_sim, from_sim };
	#[cfg(feature = "client")]
	let output_sender = Arc::new(AtomicOptionBox::none());
	#[cfg(feature = "client")]
	let output_receiver = output_sender.clone();

	thread::spawn(move || {
		run_simulation(SimulationMoveAcrossThreads {
			cb,

			#[cfg(feature = "client")]
			new_client_snapshot,

			#[cfg(feature = "server")]
			new_connection_receiver,
			#[cfg(feature = "client")]
			comms: presentation_comms,
			#[cfg(feature = "client")]
			presentation_sender: output_sender,
		})
	});

	SimControllerExternals {
		#[cfg(feature = "server")]
		new_connection_sender,
		#[cfg(feature = "client")]
		comms: sim_comms,

		#[cfg(feature = "client")]
		presentation_receiver: output_receiver,
	}
}

fn run_simulation(moved_data: SimulationMoveAcrossThreads) {
	#[allow(unused_mut)]
	let mut state = SimulationState::construct(&Rc::default(), ClientStateKind::NA);

	#[cfg(feature = "client")]
	let header = snapshot_serdes::des_new_client(&mut state, moved_data.new_client_snapshot).unwrap();

	#[cfg(feature = "server")]
	let tick_info = TickInfo::new(0, 0);
	#[cfg(feature = "client")]
	let tick_info = TickInfo::new(header.tick_id_snapshot, header.fast_forward_ticks);

	let mut sim = SimControllerInternals {
		ctx: GameContext {
			state,
			tick: tick_info,
			diff: DiffSerializer::default(),
		},

		cb: moved_data.cb,

		#[cfg(feature = "server")]
		input_history: HashMap::new(),
		#[cfg(feature = "client")]
		input_history: InternalInputHistory::default(),

		#[cfg(feature = "server")]
		new_connection_receiver: moved_data.new_connection_receiver,
		#[cfg(feature = "server")]
		comms: HashMap::new(),
		#[cfg(feature = "client")]
		comms: moved_data.comms,
		#[cfg(feature = "client")]
		output_sender: moved_data.presentation_sender,

		#[cfg(feature = "server")]
		net_events: VecDeque::from([UnrollbackableNetEvent::ServerStart]),

		#[cfg(feature = "client")]
		local_client_id: header.client_id,
	};

	#[cfg(feature = "client")]
	{
		let tick_id_fast_forward = sim.ctx.tick.id_cur + header.fast_forward_ticks;
		generate_bogus_inputs(&mut sim.input_history.ticks, header.fast_forward_ticks);
		sim.simulate(tick_id_fast_forward);
	}

	loop {
		sim.scheduled_tick_loop();
	}
}

impl SimControllerInternals {
	fn scheduled_tick_loop(&mut self) {
		if TRACE_TICK_ADVANCEMENT {
			debug!("begin scheduled tick @{:?}", self.ctx.tick);
		}

		//remember tick.id_cur is the number of completed ticks. the
		//target/goal of this iteration of the loop is to simulate 1
		//more tick than has currently finished simulating
		let tick_id_target = self.ctx.tick.id_cur + 1;

		/*
		server to client tick signaling scheme:
		- send separate state diff packets for simulation-driven vs. net events
		- when server executes net events, it is considered to only be the first half of a tick
		- first thing written to every state diff packet (both types) is the corresponding tick id.
		  client will roll back to this id upon arrival. each rollback decrements tick_id_cur
		- second thing written, if applicable, is a diff op signaling that this tick is either net events or at consensus.
		  if client receives net events, do not increment any of the 3 tick id's.
		  if client receives at consensus simulation-driven, increment both tick id's. pop the oldest element from input_history.
		  otherwise assume normal acked simulation-driven. increment id_cur
		- server must not send simulation-driven packet to clients whose input hasn't been acked for that tick (skip them).
		  if client receives simulation-driven packet, it is assumed to acknowledge client's input.
		  consensus packets (including net events) will never be skipped because consensus guarantees all inputs of that tick are acked
		  note that a tick can become consensus/finalized without all client inputs, due to waiting for inputs too long/timeout
		- after all state diff packets are processed, client then locally simulates/predicts up to id_target
		*/

		self.scheduled_tick(tick_id_target);

		if TRACE_TICK_ADVANCEMENT {
			debug!("end scheduled tick");
		}

		let next_tick_time = self.ctx.tick.get_now();

		let now = Instant::now();
		if next_tick_time > now {
			//unfortunately this is blocking, so using repl console on this thread
			//is probably a no go. not that you could do much anyway since it's
			//written in rust. also seems to sleep too long+cause clock drift if
			//tick rate is fast?
			thread::sleep(next_tick_time - now);
		} /* else if now > next_tick_time {
		//simulation tick is running behind. possible death spiral.
		//intentionally not handling this because game is unplayable
		//and player will just quit
		}*/
	}
}

//new client will attempt to rapidly fast forward
//from id_consensus to id_cur, so it won't have time
//to populate real input states. generate a bunch of
//bogus ones locally without transmitting to avoid
//wasting bandwidth. it is expected to start sending
//inputs at id_cur. the +1 is so that there is always
//at least 1 last known input, in the event that a
//client hasn't sent anything at all since the last
//consensus tick
fn generate_bogus_inputs(inputs: &mut VecDeque<InputState>, amount: TickID) {
	inputs.extend((0..amount + 1).map(|_| InputState::default()));
}
