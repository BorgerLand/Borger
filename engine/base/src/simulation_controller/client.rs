use super::*;
use crate::diff_des;
use crate::diff_ser::ser_tx_input_diff;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::presentation_state::{CloneToPresentationState, SimulationOutput};
use crate::simulation_state::InputState;
use crate::tick::TickID;
use crate::tick::TickType;
use log::debug;
use std::collections::VecDeque;
use std::i16;
use std::sync::atomic::Ordering;

//how many calibration samples to take before drawing conclusions
const OFFSET_BUFFER_SIZE: usize = 20;

//what constitutes a stable ping (+- half this amount)
const JITTER_TOLERANCE: Duration = Duration::from_millis(50);

//how far behind/ahead of the server the client can be before
//triggering recalibration (+- this amount, NOT HALF)
//note 30hz = 1000/30 = 33.3333 ms
const OFFSET_TOLERANCE: Duration = Duration::from_millis(100);

impl SimControllerInternals {
	//receive and process data from client's presentation thread
	pub(super) fn scheduled_tick_impl(&mut self, tick_id_target: TickID) {
		let rx_buffers = self.propogate_input(true);
		let offset = self.reconcile(tick_id_target, rx_buffers);

		if tick_id_target > self.ctx.tick.id_consensus {
			debug_assert_eq!(offset, 0);
			self.simulate(tick_id_target);
		} else {
			//received consensus tick that client hasn't simulated yet.
			//client is running very behind. fast forward
			debug_assert_eq!(self.ctx.tick.id_consensus, self.ctx.tick.id_cur);
			debug_assert_eq!(offset, self.ctx.tick.id_consensus - tick_id_target);

			if self.initial_calibration {
				//attempt to account for the time in between the server
				//sending the state snapshot and however long it took the
				//simulation thread to start running
				self.ctx.tick.recalibrate(-(offset as i16));
			}

			if TRACE_TICK_ADVANCEMENT {
				debug!("fell too far behind server. timed out {} ticks ago", offset);
			}
		}

		//ping must be stable in order to recalibrate accurately.
		//note that because ping is only sampled per tick, the data
		//can be somewhat course and imprecise (to the nearest
		//SIM_DT)
		if self.calibration_samples.len() == OFFSET_BUFFER_SIZE
			&& get_jitter(&self.calibration_samples) < JITTER_TOLERANCE
		{
			let average_offset = self.calibration_samples.iter().sum::<i16>() / OFFSET_BUFFER_SIZE as i16;
			if self.initial_calibration
				|| TickInfo::get_duration(average_offset.abs() as TickID) >= OFFSET_TOLERANCE
			{
				if TRACE_TICK_ADVANCEMENT {
					debug!("recalibrating by {} ticks", average_offset);
				}

				self.calibration_samples.clear();
				self.initial_calibration = false;

				if average_offset == 0 {
					return;
				}

				self.ctx.tick.recalibrate(average_offset);

				if average_offset < 0 {
					for _ in average_offset..0 {
						self.propogate_input(false);
						self.simulate(self.ctx.tick.id_cur + 1);
					}
				}
				//else not much we can do here except freeze the simulation.
				//can't rewind because inputs have already been sent.
				//theoretically should not be frozen any longer than
				//INPUT_TOO_EARLY or else the server would have kicked you.
				//being too early is also be logically impossible unless the
				//client isn't sleeping as much as it should between ticks,
				//or cheating
			}
		}

		//whatever state the simulation is in at the end of the
		//scheduled tick, render it. note there is no guarantee
		//that every simulation tick is rendered, depending on
		//whether presentation tick is able to keep up with SIM_DT
		self.output_sender.store(
			Some(Box::new(SimulationOutput {
				time: self.ctx.tick.get_now(),
				local_client_idx: self
					.ctx
					.state
					.clients
					.random_access(self.local_client_id)
					.unwrap(),
				state: self.ctx.state.clone_to_presentation(),
			})),
			Ordering::AcqRel,
		);
	}

	fn propogate_input(&mut self, read_comms: bool) -> Vec<Vec<u8>> {
		let mut input_is_late = true;
		let mut new_input = InputState::default();
		let mut rx_buffers: Vec<Vec<u8>> = Vec::new();

		if read_comms {
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
					PresentationToSimCommand::Abort => {
						panic!("Simulation received abort signal");
					}
				};
			}
		}

		if input_is_late {
			new_input = (self.cb.input_predict_late)(
				&self.input_history.entries.back().unwrap().input,
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

		//send to server
		self.comms
			.to_presentation
			.send(SimToPresentationCommand::InputDiff(diff.tx_end_tick().unwrap()))
			.unwrap();

		rx_buffers
	}

	//returns the number of buffers/ticks received that this client
	//hasn't simulated yet. anything other than 0 indicates client's
	//simulation is running very far behind
	fn reconcile(&mut self, tick_id_target: TickID, received: Vec<Vec<u8>>) -> TickID {
		//build the list of buffers that will actually be reconciled.
		//because the server can jump back in time and overwrite its own
		//previous predictions, there is a risk of seeing other clients'
		//players appear to jump backwards. this loop blocks buffers from
		//reconciling until it's proven that time has advanced forward
		//(tick_id_received >= self.ctx.tick.id_received)
		let mut safe_buffers = Vec::new();

		//while building the list, simulate what would happen to
		//tick.id_consensus if the received buffer were to go through
		let mut tick_id_consensus_simulated = self.ctx.tick.id_consensus;

		for buffer in received {
			let buffer_iter = &mut buffer.iter().cloned();
			let tick_type = TickType::des_rx(buffer_iter).unwrap();
			let tick_id_received = match tick_type {
				TickType::NetEvents => tick_id_consensus_simulated,
				TickType::Consensus => {
					tick_id_consensus_simulated += 1;
					tick_id_consensus_simulated - 1
				}
				TickType::Predicted => TickID::des_rx(buffer_iter).unwrap(),
			};

			if tick_id_received >= self.ctx.tick.id_received {
				self.ctx.tick.id_received = tick_id_received;
				safe_buffers.append(&mut self.ctx.tick.received_buffers);
				safe_buffers.push(buffer);
			} else {
				//not enough buffers have been received yet to advance the
				//simulation forward. store the packet for later reconciliation
				self.ctx.tick.received_buffers.push(buffer);
			}
		}

		let mut predicted_reconcile_amount = 0;
		let mut consensus_reconcile_amount = 0;
		let mut consensus_timeout_amount = 0;
		let mut input_underflow = 0;

		//now do it for real
		for buffer in safe_buffers {
			let buffer = &mut buffer.into_iter();
			let mut take_calibration_sample = false;
			let tick_id_received;

			let tick_type = TickType::des_rx(buffer).unwrap();
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
					let input_acked = bool::des_rx(buffer).unwrap();
					tick_id_received = self.ctx.tick.id_consensus;
					self.rollback(tick_id_received);

					predicted_reconcile_amount = 0;
					consensus_reconcile_amount += 1;
					self.ctx.tick.id_consensus += 1;
					self.ctx.tick.id_cur += 1;

					if self.input_history.entries.len() >= 2 {
						self.input_history.entries.pop_front().unwrap();

						if input_acked {
							take_calibration_sample = self.input_history.entries.front().unwrap().ping;
						}
					} else {
						//received consensus tick that client hasn't simulated yet.
						//client is running very behind. let the remaining stale
						//input stay in the buffer to uphold the len() > 0 invariant.
						//each time this happens inside the reconciliation loop, the
						//number of offset ticks to recalibrate by increments
						debug_assert_eq!(input_acked, false);
						input_underflow += 1;

						//can't skip sending any inputs even after timeout. by
						//leaving the diff blank, every input will be the same.
						//server will just discard these and decrement this client's
						//.timed_out
						self.comms
							.to_presentation
							.send(SimToPresentationCommand::InputDiff(Vec::default()))
							.unwrap();
					}

					if !input_acked {
						consensus_timeout_amount += 1;
					}
				}
				TickType::Predicted => {
					tick_id_received = TickID::des_rx(buffer).unwrap();
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

					take_calibration_sample = *associated_ping;
					*associated_ping = false;
				}
			};

			if take_calibration_sample {
				//measure ping - see comment in struct InternalInputEntry

				//example:
				//input_rtt_ping = 5 ticks (divide by 2, assume it takes 2.5 ticks to travel from client to server)
				//server_offset_ping = -8 ticks (negative means late, positive would mean early)
				//offset_estimate = round(input_rtt_ping / 2) + server_offset_ping = round(5 / 2) + -8 = -5 ticks
				//client is 5 ticks behind of server in real world time, must speed up
				//positive number would mean ahead of server, must slow down
				let input_rtt_ping = (tick_id_target + input_underflow - tick_id_received - 1) as i16;
				let server_offset_ping = i16::des_rx(buffer).unwrap();
				let offset_estimate = (input_rtt_ping + 1) / 2 + server_offset_ping;

				self.calibration_samples.push_back(offset_estimate);
				if self.calibration_samples.len() > OFFSET_BUFFER_SIZE {
					self.calibration_samples.pop_front();
				}
			}

			//yes it is correct to ser+des at the same
			//time. if the server is sending a prediction,
			//all changes need to be copied to the rollback
			//buffer in case the server has predicted wrong
			self.ctx.diff.rollback_begin_tick(tick_type);
			diff_des::des_rx_state(&mut self.ctx.state, buffer, &mut self.ctx.diff).unwrap();
			self.ctx.diff.rollback_end_tick();
		}

		if consensus_timeout_amount > 0 {
			//data is borked from large lag spike
			self.calibration_samples.clear();
			if TRACE_TICK_ADVANCEMENT {
				debug!("{} ticks experienced consensus timeout", consensus_timeout_amount);
			}
		}

		if TRACE_TICK_ADVANCEMENT {
			//note this number does not count repeats. eg. if 2
			//different buffers both contain tick 100 only 1 will
			//be counted. caused by predicted_reconcile_amount = 0
			let total_reconcile_amount = consensus_reconcile_amount + predicted_reconcile_amount;
			if total_reconcile_amount > 0 {
				debug!(
					"reconcile {} ticks ({} consensus+{} predicted)",
					total_reconcile_amount, consensus_reconcile_amount, predicted_reconcile_amount,
				);
			}
		}

		input_underflow
	}
}

fn get_jitter(samples: &VecDeque<i16>) -> Duration {
	let min = samples.iter().copied().fold(i16::MAX, i16::min);
	let max = samples.iter().copied().fold(i16::MIN, i16::max);
	let jitter = TickInfo::get_duration((max - min).abs() as TickID);
	jitter
}
