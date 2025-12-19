use crate::context::{GameContext, Impl};
use crate::diff_des;
use crate::simulation_controller::{SimControllerInternals, TRACE_TICK_ADVANCEMENT};
use crate::simulation_state::{InputState, get_owned_client_mut};
use crate::tick::{TickID, TickType};
use crate::untracked::UntrackedState;
use log::debug;
use std::collections::VecDeque;

#[cfg(feature = "server")]
use {crate::networked_types::primitive::usize32, crate::simulation_state::SimulationState};

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
			debug_assert_eq!(self.ctx.diff.rollback_buffer.len(), 0);
		}

		amount
	}

	pub(super) fn simulate(&mut self, to: TickID) {
		if to == self.ctx.tick.id_cur {
			return;
		}

		debug_assert!(to > self.ctx.tick.id_cur);
		let total_simulate_amount = to - self.ctx.tick.id_cur;

		if TRACE_TICK_ADVANCEMENT && total_simulate_amount > 0 {
			let consensus_simulate_amount = self.ctx.tick.id_consensus.saturating_sub(self.ctx.tick.id_cur);
			debug!(
				"simulate {} ticks ({} consensus+{} predicted)",
				total_simulate_amount,
				consensus_simulate_amount,
				total_simulate_amount - consensus_simulate_amount,
			);
		}

		while self.ctx.tick.id_cur < to {
			#[cfg(feature = "server")]
			let has_consensus = self.ctx.tick.has_consensus();
			#[cfg(feature = "client")]
			let has_consensus = false; //when a client is simulating a tick it is always a prediction

			#[cfg(feature = "server")]
			let input_iter = self.input_history.iter_mut();
			#[cfg(feature = "client")]
			let input_iter = iter::once((&self.local_client_id, &mut self.input_history));

			//populate the mythical client.input as defined in State.ts
			for (&client_id, input_history) in input_iter {
				let (input_prv, input_cur) = if has_consensus {
					#[cfg(feature = "server")]
					{
						self.ctx.diff.tx_toggle_client(client_id, true);

						//there should be 2 inputs for this tick on all clients,
						//where index 1 is associated with id_consensus. if not,
						//this is a forced timeout due to not receiving client's
						//inputs in time, in which case the server's prediction
						//becomes final, in order to prevent WaitForConsensus
						//stalling forever
						let prv = input_history.ticks.pop_front().unwrap();

						let cur = input_history.ticks.front().cloned().unwrap_or_else(|| {
							input_history.missing += 1;

							let finalized_prediction = (self.cb.input_predict_late)(
								&prv,
								input_history.missing as TickID,
								&self.ctx.state,
								client_id,
							);

							input_history.ticks.push_back(finalized_prediction.clone());
							finalized_prediction
						});

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
						&input_history.ticks,
						#[cfg(feature = "server")]
						self.cb.input_predict_late,
						#[cfg(feature = "server")]
						client_id,
					);

					let cur = get_input(
						self.ctx.tick.id_cur,
						&self.ctx,
						&input_history.ticks,
						#[cfg(feature = "server")]
						self.cb.input_predict_late,
						#[cfg(feature = "server")]
						client_id,
					);

					#[cfg(feature = "server")]
					self.ctx.diff.tx_toggle_client(client_id, cur.is_ok());

					(prv.unwrap_or_else(|prv| prv), cur.unwrap_or_else(|cur| cur))
				};

				let input = &mut get_owned_client_mut(&mut self.ctx.state.clients, client_id)
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

			self.ctx.diff.ser_begin_tick(
				tick_type,
				#[cfg(feature = "server")]
				self.ctx.tick.id_cur,
			);

			self.ctx.state.reset_untracked();

			//game on
			(self.cb.simulation_tick)(self.ctx.to_immediate());

			self.ctx.diff.ser_rollback_end_tick();

			#[cfg(feature = "server")]
			self.tx_all_clients();

			self.ctx.tick.id_cur += 1;
		}
	}
}

fn get_input(
	tick: TickID,
	ctx: &GameContext<Impl>,
	input_history: &VecDeque<InputState>,

	#[cfg(feature = "server")] predict_late: fn(
		/*last_known*/ &InputState,
		/*age*/ TickID,
		/*state*/ &SimulationState,
		/*client_id*/ usize32,
	) -> InputState,

	#[cfg(feature = "server")] client_id: usize32,
) -> Result<InputState, InputState> {
	let input_idx = 1 + tick - ctx.tick.id_consensus;
	match input_history.get(input_idx as usize) {
		//input has been received. server acks it
		Some(input) => Ok(input.clone()),

		//input hasn't arrived for this tick yet (not acked)
		#[cfg(feature = "server")]
		None => Err(predict_late(
			input_history.back().unwrap(),
			1 + input_idx - (input_history.len() as TickID),
			&ctx.state,
			client_id,
		)),

		//client locally will always have its own inputs. 0 latency!
		#[cfg(feature = "client")]
		None => unreachable!(),
	}
}
