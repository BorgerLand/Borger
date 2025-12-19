use crate::diff_des;
use crate::diff_ser::ser_tx_input_diff;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::presentation_state::output_presentation;
use crate::simulation_controller::SimControllerInternals;
use crate::simulation_controller::TRACE_TICK_ADVANCEMENT;
use crate::simulation_state::InputState;
use crate::simulation_state::get_owned_client;
use crate::thread_comms::*;
use crate::tick::TickID;
use crate::tick::TickType;
use log::debug;
use std::collections::VecDeque;

impl SimControllerInternals {
	//receive and process data from client's presentation thread
	pub(super) fn scheduled_tick(&mut self, tick_id_target: TickID) {
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
				self.input_history.ticks.back().unwrap(),
				1,
				&self.ctx.state,
				self.local_client_id,
			);
		}

		(self.cb.input_validate)(&mut new_input);

		let diff = &mut self.ctx.diff;
		let prv_input = &get_owned_client(&mut self.ctx.state.clients, self.local_client_id)
			.unwrap()
			.input
			.cur;
		ser_tx_input_diff(prv_input, &mut new_input, diff);

		self.input_history.ticks.push_back(new_input); //store for reconciliation
		self.comms
			.to_presentation
			.send(SimToPresentationCommand::InputDiff(diff.tx_end_tick().unwrap()))
			.unwrap(); //send to server

		self.reconcile(rx_buffers);

		if tick_id_target >= self.ctx.tick.id_cur {
			self.simulate(tick_id_target);
		} else {
			//received consensus tick that client hasn't simulated yet.
			//client is running very behind
			self.ctx.tick.recalibrate(self.ctx.tick.id_cur - tick_id_target);
		}

		output_presentation(self);
	}

	//returns how many ticks were reconciled, not including repeats
	fn reconcile(&mut self, buffers: Vec<VecDeque<u8>>) -> TickID {
		let mut predicted_reconcile_amount = 0;
		let mut consensus_reconcile_amount = 0;

		for mut buffer in buffers {
			let tick_id_received = TickID::des_rx(&mut buffer).unwrap();
			if tick_id_received < self.ctx.tick.id_cur {
				//started receiving a new timeline
				predicted_reconcile_amount = 0;
			}

			self.rollback(tick_id_received);

			let tick_type = TickType::des_rx(&mut buffer).unwrap();
			match tick_type {
				TickType::NetEvents => {
					debug_assert_eq!(tick_id_received, self.ctx.tick.id_consensus);

					if TRACE_TICK_ADVANCEMENT {
						debug!("net events triggered");
					}
				}
				TickType::Consensus => {
					debug_assert_eq!(tick_id_received, self.ctx.tick.id_consensus);

					consensus_reconcile_amount += 1;
					self.ctx.tick.id_consensus += 1;
					self.ctx.tick.id_cur += 1;

					let stale_input = self.input_history.ticks.pop_front().unwrap();
					if self.input_history.ticks.is_empty() {
						//received consensus tick that client hasn't simulated yet.
						//client is running very behind
						self.input_history.ticks.push_front((self.cb.input_predict_late)(
							&stale_input,
							1,
							&self.ctx.state,
							self.local_client_id,
						));
					}
				}
				TickType::Predicted => {
					predicted_reconcile_amount += 1;
					self.ctx.tick.id_cur += 1;
				}
			};

			//yes it is correct to ser+des at the same
			//time. if the server is sending a prediction,
			//all changes need to be copied to the rollback
			//buffer in case the server has predicted wrong
			self.ctx.diff.ser_begin_tick(tick_type);
			diff_des::des_rx_state(&mut self.ctx.state, buffer, &mut self.ctx.diff).unwrap();
			self.ctx.diff.ser_rollback_end_tick();
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
