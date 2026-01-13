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
use std::time::Duration;
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
	crate::presentation_state::SimulationOutput, crate::snapshot_serdes, atomicbox::AtomicOptionBox,
	std::sync::Arc,
};

#[cfg(feature = "client")]
mod client;
mod seek;
#[cfg(feature = "server")]
mod server;

//recommend decreasing SIM_DT with this feature on
#[cfg(feature = "server")]
const TRACE_TICK_ADVANCEMENT: bool = false;
#[cfg(feature = "client")]
const TRACE_TICK_ADVANCEMENT: bool = false;

//allow receiving client input state this early/late.
#[cfg(feature = "server")]
const INPUT_TOO_EARLY: Duration = Duration::from_secs(1); //too early = kick
const INPUT_TOO_LATE: Duration = Duration::from_secs(3); //too late = server's prediction becomes final

//communications between the simulation thread
//and the owning parent thread
pub struct SimControllerExternals {
	pub thread: thread::JoinHandle<()>,

	#[cfg(feature = "server")]
	pub new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "client")]
	pub comms: PresentationToSimChannel,

	#[cfg(feature = "client")]
	pub presentation_receiver: Arc<AtomicOptionBox<SimulationOutput>>,
}

//data that needs to be moved across threads
//during initialization of simulation thread.
//all fields must be Send
struct SimMoveAcrossThreads {
	cb: SimulationCallbacks,

	#[cfg(feature = "client")]
	new_client_snapshot: Vec<u8>,

	#[cfg(feature = "server")]
	new_connection_receiver: SyncReceiver<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "client")]
	comms: SimToPresentationChannel,
	#[cfg(feature = "client")]
	presentation_sender: Arc<AtomicOptionBox<SimulationOutput>>,
}

//on client, owned by simulation thread
pub(crate) struct SimControllerInternals {
	ctx: GameContext<Impl>,
	cb: SimulationCallbacks,

	//inputs associated with ticks that haven't reached consensus yet
	//are stored here. when more info is received ticks will be
	//resimulated using this input history. server is aware of all
	//clients' inputs; client only stores its own
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
	output_sender: Arc<AtomicOptionBox<SimulationOutput>>,

	#[cfg(feature = "server")]
	net_events: VecDeque<UnrollbackableNetEvent>,

	#[cfg(feature = "client")]
	local_client_id: usize32,
	#[cfg(feature = "client")]
	calibration_samples: VecDeque<i16>,
	#[cfg(feature = "client")]
	recalibrate_requested: bool,
}

#[derive(Default, Debug)]
struct InternalInputHistory {
	//element at index 0 corresponds to the most recently
	//completed consensus tick, and each subsequent element
	//is a progressively more recent tick. between each
	//scheduled tick (during the sleep period) it is
	//guaranteed that every client has at least one element
	//in order to compare the new input to the previous.
	//some logic asserts at least 2 elements when consensus
	//is taking place, in order to guarantee pop_front will
	//leave at least 1 left by the end of the scheduled tick
	entries: VecDeque<InternalInputEntry>,

	//how many ticks timed out and reached consensus while
	//waiting for this client's inputs. used to prevent
	//newly received inputs from pushing to the inputs
	//buffer until there are no more missing
	#[cfg(feature = "server")]
	timed_out: u32,

	//the diff received from the client is always applied to
	//this, regardless of whether there are .timed_out inputs
	#[cfg(feature = "server")]
	latest_received: InputState,
}

#[derive(Default, Debug, Clone)]
struct InternalInputEntry {
	input: InputState,

	//ping is measured in 2 different ways depending on
	//whether this is the server or client:
	//1. server: time in ticks between the tick that this entry
	//is associated with and the server's scheduled tick.id_cur
	//at the time of receiving. shipped to the corresponding
	//client as part of its state diff packet header (see the
	//signaling scheme). normally negative but can be positive
	//if client is ahead
	//2. server: rtt in ticks between the time the client sends
	//this input and the time it receives the corresponding
	//authoritative state diff from server. can only be
	//positive
	//together these 2 numbers are used to calibrate the
	//client's tick_id_target to try to match the server's
	//tick_id_target in real world time, in case they have
	//become desynced with each other. the time between
	//the client receiving the initial state snapshot and
	//actually starting the simulation always causes a
	//noticeable desync that this can fix
	#[cfg(feature = "server")]
	ping: Option<i16>, //offset
	#[cfg(feature = "client")]
	ping: bool, //whether waiting to be acked for the first time
}

pub(crate) fn init(
	cb: SimulationCallbacks,

	#[cfg(feature = "client")] new_client_snapshot: Vec<u8>,
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

	let thread = thread::spawn(move || {
		run_simulation(SimMoveAcrossThreads {
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
		thread,

		#[cfg(feature = "server")]
		new_connection_sender,
		#[cfg(feature = "client")]
		comms: sim_comms,

		#[cfg(feature = "client")]
		presentation_receiver: output_receiver,
	}
}

fn run_simulation(moved_data: SimMoveAcrossThreads) {
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
		#[cfg(feature = "client")]
		calibration_samples: VecDeque::new(),
		#[cfg(feature = "client")]
		recalibrate_requested: true,
	};

	#[cfg(feature = "client")]
	{
		let tick_id_fast_forward = sim.ctx.tick.id_cur + header.fast_forward_ticks;
		sim.input_history.generate_bogus_inputs(header.fast_forward_ticks);

		if header.fast_forward_ticks > 0 {
			sim.simulate(tick_id_fast_forward);
		}
	}

	loop {
		sim.scheduled_tick();
	}
}

impl SimControllerInternals {
	fn scheduled_tick(&mut self) {
		if TRACE_TICK_ADVANCEMENT {
			debug!("begin scheduled tick @{:?}", self.ctx.tick);
		}

		/*
		server to client tick signaling scheme:
		- when server receives a client's input, as long as it hasn't timed out yet (INPUT_TOO_LATE) it rolls back to the tick associated with it and resimulates
		- send separate state diff packets for simulation-driven vs. net events
		- when server executes net events, the associated diffs are considered to happen between id_consensus and the predicted tick after it
		- first value written to every state diff packet is the type of tick that this buffer contains state diffs for:
		  TickType::NetEvents -> client does not increment either of the tick id's. this diff is applied to the end of the most recent consensus tick to avoid rollback
		  TickType::Consensus -> client increments both tick id's. pop the oldest element from input_history
		  TickType::Predicted -> client increments id_cur only
		- second value written depends on tick type:
		  TickType::NetEvents -> nothing. no client inputs are associated with net events
		  TickType::Consensus -> whether the receiving client's inputs are acked (true) or a timeout occurred (false)
		  TickType::Predicted -> the associated tick id. clients who receive predicted ticks are guaranteed to be acked in this packet
		- the first time a client's inputs are acked, a third value is written:
		  it is the number of ticks between server's tick.id_cur at reception time and the acked input's associated tick id
		  TickType::NetEvents -> n/a, net events don't have associated inputs
		  TickType::Consensus -> only write if acked for the first time (implies previous value was true)
		  TickType::Predicted -> only write if acked for the first time
		- client will roll back to the correct id upon arrival. for NetEvents and Consensus, this means rolling back as far as possible (until the rollback buffer is empty)
		- after all state diff packets are processed, client then locally simulates/predicts up to id_target
		*/

		//remember tick.id_cur is the number of completed ticks. the
		//target/goal of this iteration of the loop is to simulate 1
		//more tick than has currently finished simulating
		self.scheduled_tick_impl(self.ctx.tick.id_cur + 1);

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
		} else if TRACE_TICK_ADVANCEMENT && now > next_tick_time {
			//simulation tick is running behind. possible death spiral.
			//intentionally not handling this because game is unplayable
			//and player will just quit
			debug!("simulation tick hiccuped");
		}
	}
}

impl InternalInputHistory {
	//new client will attempt to rapidly fast forward
	//from id_consensus to id_cur, so it won't have time
	//to populate real input states. generate a bunch of
	//bogus ones locally without transmitting to avoid
	//wasting bandwidth. it is expected to start sending
	//inputs at id_cur. the +1 is so that there is always
	//at least 1 last known input, in the event that a
	//client hasn't sent anything at all since the last
	//consensus tick
	fn generate_bogus_inputs(&mut self, amount: TickID) {
		self.entries
			.extend((0..amount + 1).map(|_| InternalInputEntry::default()));
	}
}
