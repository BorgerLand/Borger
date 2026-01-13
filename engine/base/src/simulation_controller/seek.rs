use crate::context::{GameContext, Impl};
use crate::diff_des;
use crate::simulation_controller::{InternalInputEntry, SimControllerInternals, TRACE_TICK_ADVANCEMENT};
use crate::simulation_state::get_owned_client_mut;
use crate::tick::{TickID, TickType};
use crate::untracked::UntrackedState;
use log::debug;
use std::collections::VecDeque;

#[cfg(feature = "server")]
use {
	crate::networked_types::primitive::{PrimitiveSerDes, usize32},
	crate::simulation_state::{InputState, SimulationState},
};

#[cfg(feature = "client")]
use std::iter;

impl SimControllerInternals {
	//returns how many ticks were rolled back
	pub(super) fn rollback(&mut self, to: TickID) -> TickID {
		debug_assert!(to >= self.ctx.tick.id_consensus && to <= self.ctx.tick.id_cur);

		let amount = self.ctx.tick.id_cur - to;
		if amount == 0 {
			return 0;
		}

		if TRACE_TICK_ADVANCEMENT {
			debug!(
				"rollback {} ticks ({})",
				amount,
				if to == self.ctx.tick.id_consensus {
					"full"
				} else {
					"partial"
				}
			);
		}

		while self.ctx.tick.id_cur > to {
			diff_des::des_rollback(&mut self.ctx.state, &mut self.ctx.diff.rollback_buffer).unwrap();
			self.ctx.tick.id_cur -= 1;
		}

		if self.ctx.tick.id_cur == self.ctx.tick.id_consensus {
			debug_assert!(self.ctx.diff.rollback_buffer.is_empty());
		}

		amount
	}

	//to is exclusive (do not simulate that tick)
	pub(super) fn simulate(&mut self, to: TickID) {
		debug_assert!(to > self.ctx.tick.id_cur);

		if TRACE_TICK_ADVANCEMENT {
			let total_simulate_amount = to - self.ctx.tick.id_cur;
			if total_simulate_amount > 0 {
				let consensus_simulate_amount =
					self.ctx.tick.id_consensus.saturating_sub(self.ctx.tick.id_cur);
				debug!(
					"simulate {} ticks ({} consensus+{} predicted)",
					total_simulate_amount,
					consensus_simulate_amount,
					total_simulate_amount - consensus_simulate_amount,
				);
			}
		}

		while self.ctx.tick.id_cur < to {
			#[cfg(feature = "server")]
			let has_consensus = self.ctx.tick.has_consensus();
			#[cfg(feature = "client")]
			let has_consensus = false; //when a client is simulating a tick it is always a prediction

			let tick_type = if has_consensus {
				TickType::Consensus
			} else {
				TickType::Predicted
			};

			#[cfg(feature = "server")]
			let input_iter = self.input_history.iter_mut();
			#[cfg(feature = "client")]
			let input_iter = iter::once((&self.local_client_id, &mut self.input_history));

			//populate the mythical client.input as defined in State.ts
			for (&client_id, input_history) in input_iter {
				#[cfg(feature = "server")]
				let mut buffer;
				#[cfg(feature = "server")]
				let acked_input;

				let (input_prv, input_cur) = if has_consensus {
					#[cfg(feature = "server")]
					{
						buffer = self.ctx.diff.tx_begin_tick(client_id, true);
						let buffer = buffer.as_mut().unwrap();
						tick_type.ser_tx(buffer);

						//there should be 2 inputs for this tick on all clients,
						//where index 0 is associated with the most recent
						//consensus tick and index 1 is the tick reaching
						//consensus now. if there are fewer than 2 inputs, this
						//is a forced timeout due to not receiving client's
						//inputs in time, in which case the server's prediction
						//becomes final, in order to prevent WaitForConsensus
						//game logic from stalling forever
						let prv = input_history.entries.pop_front().unwrap();

						let cur = match input_history.entries.front().cloned() {
							Some(input) => {
								acked_input = true;
								input
							}
							None => {
								//this client caused a consensus timeout
								input_history.timed_out += 1;

								let finalized_prediction = InternalInputEntry {
									input: (self.cb.input_predict_late)(
										&prv.input,
										&self.ctx.state,
										client_id,
									),
									ping: None,
								};

								input_history.entries.push_back(finalized_prediction.clone());

								acked_input = false;
								finalized_prediction
							}
						};

						acked_input.ser_tx(buffer);
						(prv, cur)
					}

					#[cfg(feature = "client")]
					{
						unreachable!()
					}
				} else {
					//tick is not at consensus so no guarantee of any inputs existing
					let prv = get_input(
						self.ctx.tick.id_cur - 1,
						&self.ctx,
						&mut input_history.entries,
						#[cfg(feature = "server")]
						self.cb.input_predict_late,
						#[cfg(feature = "server")]
						client_id,
					);

					let cur = get_input(
						self.ctx.tick.id_cur,
						&self.ctx,
						&mut input_history.entries,
						#[cfg(feature = "server")]
						self.cb.input_predict_late,
						#[cfg(feature = "server")]
						client_id,
					);

					#[cfg(feature = "server")]
					{
						acked_input = cur.is_ok();
						buffer = self.ctx.diff.tx_begin_tick(client_id, acked_input);
						if let Some(buffer) = &mut buffer {
							tick_type.ser_tx(buffer);
							self.ctx.tick.id_cur.ser_tx(buffer);
						}
					}

					(prv.unwrap_or_else(|prv| prv), cur.unwrap_or_else(|cur| cur))
				};

				let input = &mut get_owned_client_mut(&mut self.ctx.state.clients, client_id)
					.unwrap()
					.input;

				input.prv = input_prv.input;
				input.cur = input_cur.input;

				#[cfg(feature = "server")]
				if let Some(server_offset_ping) = input_cur.ping {
					//input_cur.ping.is_some() implies this is the first time
					//this input is acked, so acked_input should be true
					debug_assert_eq!(acked_input, true);
					server_offset_ping.ser_tx(buffer.unwrap());
				}
			}

			self.ctx.diff.rollback_begin_tick(tick_type);
			self.ctx.physics.start_tick();

			(self.cb.simulation_tick)(self.ctx.to_immediate()); //game on

			self.ctx.state.reset_untracked();
			self.ctx.physics.end_tick();
			self.ctx.diff.rollback_end_tick();

			#[cfg(feature = "server")]
			self.tx_all_clients();

			self.ctx.tick.id_cur += 1;
		}
	}
}

//ok = true acked input, err = generated by predict_late
fn get_input(
	tick: TickID,
	ctx: &GameContext<Impl>,
	history: &mut VecDeque<InternalInputEntry>,

	#[cfg(feature = "server")] predict_late: fn(
		/*last_known*/ &InputState,
		/*state*/ &SimulationState,
		/*client_id*/ usize32,
	) -> InputState,

	#[cfg(feature = "server")] client_id: usize32,
) -> Result<InternalInputEntry, InternalInputEntry> {
	let input_idx = 1 + tick - ctx.tick.id_consensus;
	match history.get_mut(input_idx as usize) {
		//input has been received. server acks it
		Some(entry) => {
			let clone = entry.clone();

			#[cfg(feature = "server")]
			{
				//only send the ping the first time this input is acked
				entry.ping = None;
			}

			Ok(clone)
		}

		//input hasn't arrived for this tick yet (not acked)
		#[cfg(feature = "server")]
		None => {
			let age = 1 + input_idx - (history.len() as TickID);
			let mut input = history.back().unwrap().input.clone();
			for _ in 0..age {
				input = predict_late(&input, &ctx.state, client_id);
			}

			Err(InternalInputEntry { input, ping: None })
		}

		//client locally will always have its own inputs. 0 latency!
		#[cfg(feature = "client")]
		None => unreachable!(),
	}
}
