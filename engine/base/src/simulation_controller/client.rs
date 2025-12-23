use crate::diff_des;
use crate::diff_ser::ser_tx_input_diff;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::presentation_state::output_presentation;
use crate::simulation_controller::{InternalInputEntry, SimControllerInternals, TRACE_TICK_ADVANCEMENT};
use crate::simulation_state::InputState;
use crate::thread_comms::*;
use crate::tick::TickID;
use crate::tick::TickType;
use log::debug;
use std::collections::VecDeque;

impl SimControllerInternals {
	//receive and process data from client's presentation thread
	pub(super) fn scheduled_tick_impl(&mut self, tick_id_target: TickID) {
		let mut input_is_late = true;
		let mut new_input = InputState::default();

		let mut rx_buffers: Vec<VecDeque<u8>> = Vec::new();

		while let Ok(presentation_msg) = self.comms.from_presentation.try_recv() {
			match presentation_msg {
				PresentationToSimCommand::RawInput(raw_input) => {
					//there can only be one input state per simulation tick,
					//so merge together however many the presentation thread
					//has produced
					(self.cb.input_merge)(&mut new_input, &raw_input);
					input_is_late = false;
				}
				PresentationToSimCommand::ReceiveState(buffer) => {
					rx_buffers.push(buffer);
				}
			};
		}

		if input_is_late {
			new_input = (self.cb.input_predict_late)(
				&self.input_history.entries.back().unwrap().input,
				1,
				&self.ctx.state,
				self.local_client_id,
			);
		}

		(self.cb.input_validate)(&mut new_input);

		let diff = &mut self.ctx.diff;
		let prv_input = &self.input_history.entries.back().unwrap().input;
		ser_tx_input_diff(prv_input, &mut new_input, diff);

		//store for reconciliation
		self.input_history.entries.push_back(InternalInputEntry {
			input: new_input,
			ping: true,
		});

		self.comms
			.to_presentation
			.send(SimToPresentationCommand::InputDiff(diff.tx_end_tick().unwrap()))
			.unwrap(); //send to server

		self.reconcile(tick_id_target, rx_buffers);

		if tick_id_target > self.ctx.tick.id_consensus {
			self.simulate(tick_id_target);
		} else {
			//received consensus tick that client hasn't simulated yet.
			//client is running very behind. fast forward
			debug_assert_eq!(self.ctx.tick.id_consensus, self.ctx.tick.id_cur);
			let offset = self.ctx.tick.id_consensus - tick_id_target;

			if TRACE_TICK_ADVANCEMENT {
				debug!("fell too far behind server. timed out {} ticks ago", offset);
			}

			self.ctx.tick.recalibrate(offset);

			//for each tick that the client just fast forwarded through,
			//send a blank input diff to the server. even though they've
			//already reached consensus the server still needs an input
			//for every tick in order to track association. by leaving
			//the diff blank, every input will be the same. might cause
			//some visual jank but this is ok because being multiple
			//seconds behind is bound to be janky no matter what
			for _ in 0..offset {
				self.comms
					.to_presentation
					.send(SimToPresentationCommand::InputDiff(Vec::default()))
					.unwrap(); //send to server
			}
		}

		output_presentation(self);
	}

	//returns how many ticks were reconciled, not including repeats
	fn reconcile(&mut self, mut tick_id_target: TickID, buffers: Vec<VecDeque<u8>>) -> TickID {
		let mut predicted_reconcile_amount = 0;
		let mut consensus_reconcile_amount = 0;

		for mut buffer in buffers {
			let mut measure_ping = false; //depends on whether this buffer acks client for the first time
			let tick_id_received;

			let tick_type = TickType::des_rx(&mut buffer).unwrap();
			match tick_type {
				TickType::NetEvents => {
					tick_id_received = self.ctx.tick.id_consensus;

					self.rollback(tick_id_received);

					predicted_reconcile_amount = 0;

					if TRACE_TICK_ADVANCEMENT {
						debug!("net events triggered");
					}
				}
				TickType::Consensus => {
					let input_acked = bool::des_rx(&mut buffer).unwrap();
					tick_id_received = self.ctx.tick.id_consensus;

					self.rollback(tick_id_received);

					predicted_reconcile_amount = 0;
					consensus_reconcile_amount += 1;
					self.ctx.tick.id_consensus += 1;
					self.ctx.tick.id_cur += 1;

					if self.input_history.entries.len() > 1 {
						self.input_history.entries.pop_front().unwrap();

						if input_acked {
							measure_ping = self
								.input_history
								.entries
								.front_mut()
								.map(|consensus_input| consensus_input.ping)
								.unwrap_or(false);
						}
					} else {
						//received consensus tick that client hasn't simulated yet.
						//client is running very behind. let the remaining stale
						//input stay in the buffer to uphold the len() > 0 invariant
						debug_assert_eq!(input_acked, false);
						tick_id_target += 1;
					}
				}
				TickType::Predicted => {
					tick_id_received = TickID::des_rx(&mut buffer).unwrap();

					self.rollback(tick_id_received);

					if tick_id_received < self.ctx.tick.id_cur {
						//started receiving a new timeline
						predicted_reconcile_amount = 0;
					}

					predicted_reconcile_amount += 1;
					self.ctx.tick.id_cur += 1;

					let associated_ping = &mut self.input_history.entries
						[(1 + tick_id_received - self.ctx.tick.id_consensus) as usize]
						.ping;

					measure_ping = *associated_ping;
					*associated_ping = false;
				}
			};

			//measure ping - see comment in struct InternalInputEntry
			if measure_ping {
				let input_rtt_ping = (tick_id_target - tick_id_received - 1) as i16;
				let server_offset_ping = i16::des_rx(&mut buffer).unwrap();

				//example:
				//input_rtt_ping = 3 ticks (divide by 2, assume it takes 5 ticks to travel from client to server)
				//server_offset_ping = -1 tick (negative means late, positive would mean early)
				//round(input_rtt_ping / 2) + server_offset_ping = round(3 / 2) + -1 = 1 tick
				//client is 2 ticks ahead of server in real world time, must slow down
				let offset_estimate = (input_rtt_ping + 1) / 2 + server_offset_ping;
				debug!(
					"tick id {} has offset estimate {} ({}, {})",
					tick_id_received, offset_estimate, input_rtt_ping, server_offset_ping
				);
			}

			//yes it is correct to ser+des at the same
			//time. if the server is sending a prediction,
			//all changes need to be copied to the rollback
			//buffer in case the server has predicted wrong
			self.ctx.diff.rollback_begin_tick(tick_type);
			diff_des::des_rx_state(&mut self.ctx.state, buffer, &mut self.ctx.diff).unwrap();
			self.ctx.diff.rollback_end_tick();
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
}
