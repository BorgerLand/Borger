use crate::ClientStateKind;
use crate::SimulationCallbacks;
use crate::constructors::ConstructCustomStruct;
use crate::context::{GameContext, Impl};
use crate::diff_des;
use crate::diff_ser::DiffSerializer;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::{InputState, SimulationState, get_owned_client_mut};
use crate::snapshot_serdes;
use crate::thread_comms::*;
use crate::tick::{TickID, TickInfo, TickType};
use crate::untracked::UntrackedState;
use log::debug;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc as sync_mpsc;
use wasm_thread as thread;
use web_time::Instant;

#[cfg(feature = "server")]
use {
	crate::snapshot_serdes::NewClientHeader,
	crate::tick::UnrollbackableNetEvent,
	std::collections::HashMap,
	std::sync::mpsc::{Receiver as SyncReceiver, Sender as SyncSender},
	std::time::Duration,
	tokio::sync::mpsc::UnboundedSender as AsyncSender,
};

#[cfg(feature = "client")]
use {
	crate::diff_ser::ser_tx_input_diff,
	crate::networked_types::primitive::PrimitiveSerDes,
	crate::presentation_state::{PresentationTick, output_presentation},
	crate::simulation_state::get_owned_client,
	atomicbox::AtomicOptionBox,
	std::iter,
	std::sync::Arc,
};

#[cfg(feature = "server")]
const TRACE_TICK_ADVANCEMENT: bool = false;
#[cfg(feature = "client")]
const TRACE_TICK_ADVANCEMENT: bool = false;

//allow receiving client input state this early/late.
//too early = kick
//too late = server's prediction becomes final
#[cfg(feature = "server")]
const INPUT_ARRIVAL_WINDOW: Duration = Duration::from_secs(1);

//communications between the simulation thread
//and the owning parent thread
pub struct SimulationExternals {
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
pub(crate) struct SimulationInternals {
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
	input_history: HashMap<usize32, VecDeque<InputState>>,
	#[cfg(feature = "client")]
	input_history: VecDeque<InputState>,

	#[cfg(feature = "server")]
	new_connection_receiver: SyncReceiver<AsyncSender<SimToClientCommand>>,
	#[cfg(feature = "server")]
	comms: HashMap<usize32, SimToClientChannel>,
	#[cfg(feature = "client")]
	comms: SimToPresentationChannel,
	#[cfg(feature = "client")]
	pub output_sender: Arc<AtomicOptionBox<PresentationTick>>,

	#[cfg(feature = "client")]
	pub local_client_id: usize32,
}

pub fn init(
	cb: SimulationCallbacks,

	#[cfg(feature = "client")] new_client_snapshot: VecDeque<u8>,
) -> SimulationExternals {
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

	SimulationExternals {
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

	let mut sim = SimulationInternals {
		ctx: GameContext {
			state,
			tick: tick_info,
			diff: DiffSerializer::default(),
		},

		cb: moved_data.cb,

		#[cfg(feature = "server")]
		input_history: HashMap::new(),
		#[cfg(feature = "client")]
		input_history: VecDeque::new(),

		#[cfg(feature = "server")]
		new_connection_receiver: moved_data.new_connection_receiver,
		#[cfg(feature = "server")]
		comms: HashMap::new(),
		#[cfg(feature = "client")]
		comms: moved_data.comms,
		#[cfg(feature = "client")]
		output_sender: moved_data.presentation_sender,

		#[cfg(feature = "client")]
		local_client_id: header.client_id,
	};

	#[cfg(feature = "server")]
	let mut net_events: VecDeque<UnrollbackableNetEvent> = VecDeque::new();
	#[cfg(feature = "server")]
	net_events.push_back(UnrollbackableNetEvent::ServerStart);

	#[cfg(feature = "client")]
	{
		let tick_id_fast_forward = sim.ctx.tick.id_cur + header.fast_forward_ticks;
		generate_bogus_inputs(&mut sim.input_history, header.fast_forward_ticks);
		simulate(&mut sim, tick_id_fast_forward);
	}

	loop {
		if TRACE_TICK_ADVANCEMENT {
			debug!("begin scheduled tick @{:?}", sim.ctx.tick);
		}

		//remember tick.id_cur is the number of completed ticks. the
		//target/goal of this iteration of the loop is to simulate 1
		//more tick than has currently finished simulating
		let tick_id_target = sim.ctx.tick.id_cur + 1;

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

		//receive and process data from server's main thread
		#[cfg(feature = "server")]
		{
			//in order to minimize the amount of rolling back
			//for processing non-deterministic net_events,
			//inputs need to be deserialized first. remember
			//that tick.id_consensus can only increment when
			//inputs have been received for all clients for
			//that particular tick id

			//connect event received from new_connection_receiver
			while let Ok(to_client) = sim.new_connection_receiver.try_recv() {
				let (to_sim, from_client) = sync_mpsc::channel();
				let comms = SimToClientChannel {
					to_client,
					from_client,
				};
				comms.to_client.send(SimToClientCommand::Connect(to_sim)).unwrap();
				net_events.push_back(UnrollbackableNetEvent::ClientConnect(comms));
			}

			//other events received from client-specific comms
			let mut rollback_to = sim.ctx.tick.id_cur; //oldest tick id associated with a newly received input
			for (&id, client) in sim.comms.iter() {
				while let Ok(client_msg) = client.from_client.try_recv() {
					match client_msg {
						ClientToSimCommand::ReceiveInput(input_history) => {
							let inputs = sim.input_history.get_mut(&id).unwrap();
							let associated_tick = sim.ctx.tick.id_consensus - 1 + inputs.len() as TickID;
							let mut new_input = inputs.back().unwrap().clone();

							//this occurrence of deserialization needs to be
							//treated with care because the inputs aren't
							//trusted. an evil client could otherwise crash
							//the server by sending a corrupt input
							match diff_des::des_rx_input(&mut new_input, input_history) {
								Ok(_) => {
									(sim.cb.input_validate)(&mut new_input);
									inputs.push_back(new_input);

									if associated_tick < rollback_to {
										rollback_to = associated_tick;
									}
								}
								Err(oops) => {
									//connection must be killed. this should never
									//happen unless client is attempting to cheat
									#[allow(unused_must_use)]
									//safe to ignore error. if client has disconnected, sim thread will be notified shortly
									client.to_client.send(SimToClientCommand::RequestKick(format!(
										"received corrupt input containing {:?}",
										oops
									)));
								}
							};
						}

						ClientToSimCommand::Disconnect => {
							net_events.push_back(UnrollbackableNetEvent::ClientDisconnect(id));
						}
					};
				}
			}

			//in an ideal world, input_history now contains, for
			//every client, inputs corresponding to tick id's
			//[consensus - 1, cur]. index 1 definitely refers
			//to tick.is_consensus, but the exact length of each
			//client's vec is unpredictable and depends on ping/
			//latency. it's even possible they may have too many

			let input_on_time = sim.ctx.tick.get_instant(sim.ctx.tick.id_cur);
			let input_too_new = input_on_time + INPUT_ARRIVAL_WINDOW;

			for (client_id, inputs) in sim.input_history.iter() {
				let associated_time = sim
					.ctx
					.tick
					.get_instant(sim.ctx.tick.id_consensus - 1 + inputs.len() as TickID);
				if associated_time >= input_too_new {
					#[allow(unused_must_use)]
					//safe to ignore error. if client has disconnected, sim thread will be notified shortly
					sim.comms
						.get(client_id)
						.unwrap()
						.to_client
						.send(SimToClientCommand::RequestKick(format!(
							"received inputs too far into the future (> {:?})",
							INPUT_ARRIVAL_WINDOW
						)));
				}
			}

			//it is possible that the rollback amount is 0 ticks,
			//in the case that the server is empty, hasn't
			//received any inputs, or the (unideal) scenario
			//that all clients' tick.id_cur is at/ahead of the
			//server's
			let rollback_amount = rollback(&mut sim, rollback_to);
			debug_assert_eq!(rollback_to, sim.ctx.tick.id_cur);

			//this may trigger even more rolling back
			trigger_net_events(&mut sim, &mut net_events, rollback_amount);

			if sim.ctx.tick.id_cur == sim.ctx.tick.id_consensus {
				//tick.id_consensus is calculated based on
				//what is the oldest tick id for which an input
				//has been received from all clients
				let max_advance = tick_id_target - sim.ctx.tick.id_consensus;
				sim.ctx.tick.id_consensus += sim
					.input_history
					.values()
					.map(|inputs| inputs.len() - 1)
					.min()
					.map(|min| min as TickID)
					.unwrap_or(max_advance) //if there are no clients, insta-advance to tick_id_target
					.min(max_advance) //if all clients are living in the future, prevent them from fast forwarding the whole server
			}
		}

		//receive and process data from client's presentation thread
		#[cfg(feature = "client")]
		{
			let mut input_is_late = true;
			let mut new_input = InputState::default();

			let mut rx_buffers: Vec<VecDeque<u8>> = Vec::new();

			while let Ok(presentation_msg) = sim.comms.from_presentation.try_recv() {
				match presentation_msg {
					PresentationToSimCommand::RawInput(raw_input) => {
						//there can only be one input state per simulation tick,
						//so merge together however many the presentation thread
						//has produced
						(sim.cb.input_merge)(&mut new_input, &raw_input);
						input_is_late = false;
					}
					PresentationToSimCommand::ReceiveState(buffer) => {
						rx_buffers.push(buffer);
					}
				};
			}

			if input_is_late {
				new_input = (sim.cb.input_predict_late)(
					sim.input_history.back().unwrap(),
					1,
					&sim.ctx.state,
					sim.local_client_id,
				);
			}

			(sim.cb.input_validate)(&mut new_input);

			let diff = &mut sim.ctx.diff;
			let prv_input = &get_owned_client(&mut sim.ctx.state.clients, sim.local_client_id)
				.unwrap()
				.input
				.cur;
			ser_tx_input_diff(prv_input, &mut new_input, diff);

			sim.input_history.push_back(new_input); //store for reconciliation
			sim.comms
				.to_presentation
				.send(SimToPresentationCommand::InputDiff(diff.tx_end_tick().unwrap()))
				.unwrap(); //send to server

			reconcile(&mut sim, rx_buffers);
		}

		simulate(&mut sim, tick_id_target);

		#[cfg(feature = "client")]
		output_presentation(&mut sim);

		if TRACE_TICK_ADVANCEMENT {
			debug!("end scheduled tick");
		}

		let next_tick_time = sim.ctx.tick.get_now();

		let now = Instant::now();
		if next_tick_time > now {
			//unfortunately this is blocking, so using repl console on this thread
			//is probably a no go. not that you could do much anyway since it's
			//written in rust. also seems to sleep too long+cause clock drift if
			//tick rate is fast?
			thread::sleep(next_tick_time - now);
		} /* else if now > next_tick_time {
		//simulation tick is running behind. possible death spiral
		}*/
	}
}

//returns how many ticks were rolled back
fn rollback(sim: &mut SimulationInternals, to: TickID) -> TickID {
	debug_assert!(to >= sim.ctx.tick.id_consensus && to <= sim.ctx.tick.id_cur);

	let amount = sim.ctx.tick.id_cur - to;
	if amount == 0 {
		return 0;
	}

	if TRACE_TICK_ADVANCEMENT {
		debug!(
			"rollback {} ticks ({})",
			amount,
			if to == sim.ctx.tick.id_consensus {
				"full"
			} else {
				"partial"
			}
		);
	}

	while sim.ctx.tick.id_cur > to {
		diff_des::des_rollback(&mut sim.ctx.state, &mut sim.ctx.diff.rollback_buffer).unwrap();
		sim.ctx.tick.id_cur -= 1;
	}

	if sim.ctx.tick.id_cur == sim.ctx.tick.id_consensus {
		debug_assert_eq!(sim.ctx.diff.rollback_buffer.len(), 0);
	}

	amount
}

#[cfg(feature = "server")]
fn trigger_net_events(
	sim: &mut SimulationInternals,
	net_events: &mut VecDeque<UnrollbackableNetEvent>,
	mut rollback_amount: TickID,
) {
	if net_events.is_empty() {
		return;
	}

	if TRACE_TICK_ADVANCEMENT {
		debug!("net events triggered");
	}

	//full rollback required
	rollback_amount += rollback(sim, sim.ctx.tick.id_consensus);

	//split this tick into 2 halves:
	//state changes caused by net events, and
	//state changes caused by the simulation
	for &client in sim.comms.keys() {
		sim.ctx.diff.tx_toggle_client(client, true);
	}

	sim.ctx
		.diff
		.ser_begin_tick(TickType::NetEvents, sim.ctx.tick.id_cur);

	while let Some(event) = net_events.pop_front() {
		match event {
			UnrollbackableNetEvent::ServerStart => {
				(sim.cb.on_server_start)(&mut sim.ctx.state, sim.ctx.diff.to_consensus());
			}

			UnrollbackableNetEvent::ClientConnect(comms) => {
				let new_client_id = sim
					.ctx
					.state
					.clients
					.add_with_client_owned(ClientStateKind::Owned, &mut sim.ctx.diff)
					.0;

				//game logic-specific
				(sim.cb.on_client_connect)(
					&mut sim.ctx.state,
					new_client_id,
					sim.ctx.tick.id_cur,
					sim.ctx.diff.to_consensus(),
				);

				let snapshot = snapshot_serdes::ser_new_client(
					&sim.ctx.state,
					NewClientHeader {
						client_id: new_client_id,
						tick_id_snapshot: sim.ctx.tick.id_cur,
						fast_forward_ticks: rollback_amount,
					},
				);

				#[allow(unused_must_use)]
				//safe to ignore error. if client has disconnected, sim thread will be notified shortly
				comms.to_client.send(SimToClientCommand::SendState(snapshot));

				sim.comms.insert(new_client_id, comms);
				generate_bogus_inputs(
					sim.input_history.entry(new_client_id).or_default(),
					rollback_amount,
				);
				sim.ctx.diff.on_connect(new_client_id, sim.ctx.tick.id_cur);
			}

			UnrollbackableNetEvent::ClientDisconnect(old_client_id) => {
				sim.ctx.diff.on_disconnect(old_client_id);
				sim.input_history.remove(&old_client_id).unwrap();
				sim.comms.remove(&old_client_id).unwrap();

				//game logic-specific
				(sim.cb.on_client_disconnect)(
					&mut sim.ctx.state,
					old_client_id,
					sim.ctx.tick.id_cur,
					sim.ctx.diff.to_consensus(),
				);

				sim.ctx
					.state
					.clients
					.remove(old_client_id, &mut sim.ctx.diff)
					.unwrap();
			}
		};
	}

	tx_all_clients(sim);
}

//returns how many ticks were reconciled, not including repeats
#[cfg(feature = "client")]
fn reconcile(sim: &mut SimulationInternals, buffers: Vec<VecDeque<u8>>) -> TickID {
	let mut predicted_reconcile_amount = 0;
	let mut consensus_reconcile_amount = 0;

	for mut buffer in buffers {
		let tick_id_received = TickID::des_rx(&mut buffer).unwrap();
		if tick_id_received < sim.ctx.tick.id_cur {
			//started receiving a new timeline
			predicted_reconcile_amount = 0;
		}

		rollback(sim, tick_id_received);

		let tick_type = TickType::des_rx(&mut buffer).unwrap();
		match tick_type {
			TickType::NetEvents => {
				debug_assert_eq!(tick_id_received, sim.ctx.tick.id_consensus);

				if TRACE_TICK_ADVANCEMENT {
					debug!("net events triggered");
				}
			}
			TickType::Consensus => {
				debug_assert_eq!(tick_id_received, sim.ctx.tick.id_consensus);

				consensus_reconcile_amount += 1;
				sim.ctx.tick.id_consensus += 1;
				sim.ctx.tick.id_cur += 1;

				sim.input_history.pop_front();
				debug_assert!(sim.input_history.len() >= 2);
			}
			TickType::Predicted => {
				predicted_reconcile_amount += 1;
				sim.ctx.tick.id_cur += 1;
			}
		};

		//yes it is correct to ser+des at the same
		//time. if the server is sending a prediction,
		//all changes need to be copied to the rollback
		//buffer in case the server has predicted wrong
		sim.ctx.diff.ser_begin_tick(tick_type);
		diff_des::des_rx_state(&mut sim.ctx.state, buffer, &mut sim.ctx.diff).unwrap();
		sim.ctx.diff.ser_rollback_end_tick();
	}

	let total_reconcile_amount = consensus_reconcile_amount + predicted_reconcile_amount;
	if TRACE_TICK_ADVANCEMENT && total_reconcile_amount > 0 {
		debug!(
			"reconcile {} ticks ({} consensus+{} predicted)",
			total_reconcile_amount, consensus_reconcile_amount, predicted_reconcile_amount,
		);
	}

	total_reconcile_amount
}

//returns how many ticks were simulated
fn simulate(sim: &mut SimulationInternals, to: TickID) -> TickID {
	if to == sim.ctx.tick.id_cur {
		return 0;
	}

	debug_assert!(to > sim.ctx.tick.id_cur);
	let total_simulate_amount = to - sim.ctx.tick.id_cur;

	if TRACE_TICK_ADVANCEMENT && total_simulate_amount > 0 {
		let consensus_simulate_amount = sim.ctx.tick.id_consensus.saturating_sub(sim.ctx.tick.id_cur);
		debug!(
			"simulate {} ticks ({} consensus+{} predicted)",
			total_simulate_amount,
			consensus_simulate_amount,
			total_simulate_amount - consensus_simulate_amount,
		);
	}

	while sim.ctx.tick.id_cur < to {
		#[cfg(feature = "server")]
		let has_consensus = sim.ctx.tick.has_consensus();
		#[cfg(feature = "client")]
		let has_consensus = false; //when a client is simulating a tick it is always a prediction

		#[cfg(feature = "server")]
		let input_iter = sim.input_history.iter_mut();
		#[cfg(feature = "client")]
		let input_iter = iter::once((&sim.local_client_id, &mut sim.input_history));

		//populate the mythical client.input as defined in State.ts
		for (&client_id, input_history) in input_iter {
			let (input_prv, input_cur) = if has_consensus {
				#[cfg(feature = "server")]
				sim.ctx.diff.tx_toggle_client(client_id, true);

				//2 inputs are guaranteed to exist for this tick
				//on all clients (remember index 1 is associated
				//with id_consensus). all clients having at least 2
				//inputs is what makes a tick at consensus in the
				//first place
				(input_history.pop_front().unwrap(), input_history[0].clone())
			} else {
				//tick is at consensus so no guarantee of any inputs existing
				let prv = get_input(
					sim.ctx.tick.id_cur - 1,
					&sim.ctx,
					input_history,
					#[cfg(feature = "server")]
					sim.cb.input_predict_late,
					#[cfg(feature = "server")]
					client_id,
				);

				let cur = get_input(
					sim.ctx.tick.id_cur,
					&sim.ctx,
					input_history,
					#[cfg(feature = "server")]
					sim.cb.input_predict_late,
					#[cfg(feature = "server")]
					client_id,
				);

				#[cfg(feature = "server")]
				sim.ctx.diff.tx_toggle_client(client_id, cur.is_ok());

				(prv.unwrap_or_else(|prv| prv), cur.unwrap_or_else(|cur| cur))
			};

			let input = &mut get_owned_client_mut(&mut sim.ctx.state.clients, client_id)
				.unwrap()
				.input;

			input.prv = input_prv;
			input.cur = input_cur;
		}

		let tick_type = if has_consensus {
			TickType::Consensus
		} else {
			TickType::Predicted
		};

		sim.ctx.diff.ser_begin_tick(
			tick_type,
			#[cfg(feature = "server")]
			sim.ctx.tick.id_cur,
		);

		sim.ctx.state.reset_untracked();

		//game on
		(sim.cb.simulation_tick)(sim.ctx.to_immediate());

		sim.ctx.diff.ser_rollback_end_tick();

		#[cfg(feature = "server")]
		tx_all_clients(sim);

		sim.ctx.tick.id_cur += 1;
	}

	total_simulate_amount
}

#[cfg(feature = "server")]
fn tx_all_clients(sim: &mut SimulationInternals) {
	for (&client_id, comms) in sim.comms.iter() {
		if let Some(diff) = sim.ctx.diff.tx_end_tick(client_id) {
			#[allow(unused_must_use)]
			//safe to ignore error. if client has disconnected, sim thread will be notified shortly
			comms.to_client.send(SimToClientCommand::SendState(diff));
		}
	}
}

fn generate_bogus_inputs(inputs: &mut VecDeque<InputState>, amount: TickID) {
	//new client will attempt to rapidly fast forward
	//from id_consensus to id_cur, so it won't have time
	//to populate real input states. generate a bunch of
	//bogus ones locally without transmitting to avoid
	//wasting bandwidth. it is expected to start sending
	//inputs at id_cur. the +1 is so that there is always
	//at least 1 last known input, in the event that a
	//client hasn't sent anything at all since the last
	//consensus tick
	inputs.extend((0..amount + 1).map(|_| InputState::default()));
}

fn get_input(
	tick: TickID,
	ctx: &GameContext<Impl>,
	client_input_history: &VecDeque<InputState>,

	#[cfg(feature = "server")] predict_late: fn(
		/*last_known*/ &InputState,
		/*age*/ TickID,
		/*state*/ &SimulationState,
		/*client_id*/ usize32,
	) -> InputState,

	#[cfg(feature = "server")] client_id: usize32,
) -> Result<InputState, InputState> {
	let input_idx = 1 + tick - ctx.tick.id_consensus;
	match client_input_history.get(input_idx as usize) {
		//input has been received. server acks it
		Some(input) => Ok(input.clone()),

		//input hasn't arrived for this tick yet (not acked)
		#[cfg(feature = "server")]
		None => Err(predict_late(
			client_input_history.back().unwrap(),
			1 + input_idx - (client_input_history.len() as TickID),
			&ctx.state,
			client_id,
		)),

		//client locally will always have its own inputs. 0 latency!
		#[cfg(feature = "client")]
		None => unreachable!(),
	}
}
