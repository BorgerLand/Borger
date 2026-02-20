use super::*;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::snapshot_serdes;
use crate::snapshot_serdes::NewClientHeader;
use crate::tick::{TickID, TickInfo, TickType, UnrollbackableNetEvent};
use crate::{ClientStateKind, diff_des};
use log::debug;
use std::sync::mpsc as sync_mpsc;

//allow receiving client input state this early/late.
const INPUT_TOO_EARLY: Duration = Duration::from_millis(1000); //too early = kick
const INPUT_TOO_LATE: Duration = Duration::from_millis(1500); //too late = server's prediction becomes final

impl SimControllerInternals {
	//receive and process data from server's main thread
	pub(super) fn scheduled_tick_impl(&mut self, tick_id_target: TickID) {
		//in order to minimize the amount of rolling back
		//for processing non-deterministic net_events,
		//inputs need to be deserialized first. remember
		//that tick.id_consensus can only increment when
		//inputs have been received for all clients for
		//that particular tick id

		//connect event received from new_connection_receiver
		while let Ok(to_client) = self.new_connection_receiver.try_recv() {
			let (to_sim, from_client) = sync_mpsc::channel();
			let comms = SimToClientChannel {
				to_client,
				from_client,
			};
			comms.to_client.send(SimToClientCommand::Connect(to_sim)).unwrap();
			self.net_events
				.push_back(UnrollbackableNetEvent::ClientConnect(comms));
		}

		//if client has an input for this tick id already, it's
		//probably trying to cheat, so kick it
		let tick_id_too_early = self.ctx.tick.id_cur + TickInfo::get_ticks(INPUT_TOO_EARLY);

		//if client doesn't have an input for this tick id yet,
		//put it in timeout mode
		let tick_id_too_late = self
			.ctx
			.tick
			.id_cur
			.saturating_sub(TickInfo::get_ticks(INPUT_TOO_LATE));

		//if client is in timeout mode and has an input for this
		//tick id, disable timeout mode and allow influencing the
		//simulation again
		let tick_id_caught_up = self
			.ctx
			.tick
			.id_cur
			.saturating_sub(TickInfo::get_ticks(INPUT_TOO_LATE / 2));

		let mut rollback_to = self.ctx.tick.id_cur; //oldest tick id associated with a newly received input
		//other events received from client-specific comms
		for (id, history) in self.input_history.iter_mut() {
			let mut tick_id_associated =
				self.ctx.tick.id_consensus + history.entries.len() as TickID - 2 - history.timed_out_ticks;

			let client_comms = self.comms.get(id).unwrap();
			while let Ok(client_msg) = client_comms.from_client.try_recv() {
				match client_msg {
					ClientToSimCommand::ReceiveInput(ser_rx_buffer) => {
						let mut new_input = history.latest_received.clone();

						//this occurrence of deserialization needs to be
						//treated with care because the inputs aren't
						//trusted. an evil client could otherwise crash
						//the server by sending a corrupt input
						match diff_des::des_rx_input(&mut new_input, ser_rx_buffer.into_iter()) {
							Ok(_) => {
								tick_id_associated += 1;
								(self.cb.input_validate)(&mut new_input);

								if history.is_timed_out && tick_id_associated >= tick_id_caught_up {
									history.is_timed_out = false;
									if TRACE_TICK_ADVANCEMENT {
										debug!(
											"pinning consensus tick to help client {} finish catching up",
											id
										);
									}
								}

								if history.timed_out_ticks == 0 {
									history.entries.push_back(InternalInputEntry {
										input: new_input.clone(),
										ping: Some(
											tick_id_associated.wrapping_sub(self.ctx.tick.id_cur) as i16
										),
									});

									history.is_timed_out = false; //sanity check
									rollback_to = rollback_to.min(tick_id_associated);
								} else {
									//this tick has already reched consensus so the
									//input is no longer needed for simulation
									history.timed_out_ticks -= 1;
									if TRACE_TICK_ADVANCEMENT && history.timed_out_ticks == 0 {
										debug!("client {} exited timeout state", id);
									}
								}

								history.latest_received = new_input;
							}
							Err(oops) => {
								//connection must be killed. this should never
								//happen unless client is attempting to cheat
								#[allow(unused_must_use)]
								//safe to ignore error. if client has disconnected, sim thread will be notified shortly
								client_comms
									.to_client
									.send(SimToClientCommand::RequestKick(format!(
										"received corrupt input containing {:?}",
										oops
									)));
							}
						};
					}

					ClientToSimCommand::Disconnect => {
						self.net_events
							.push_back(UnrollbackableNetEvent::ClientDisconnect(*id));
					}
				};
			}

			if history.timed_out_ticks == 0 {
				if tick_id_associated >= tick_id_too_early {
					//handle too early (or possibly the server is death spiraling?)
					debug_assert_ne!(history.entries.len(), 0);

					#[allow(unused_must_use)]
					//safe to ignore error. if client has disconnected, sim thread will be notified shortly
					self.comms
						.get(id)
						.unwrap()
						.to_client
						.send(SimToClientCommand::RequestKick(format!(
							"received inputs too far into the future (>= {:?})",
							INPUT_TOO_EARLY
						)));
				} else if tick_id_associated < tick_id_too_late {
					//handle too late: enter timeout mode
					history.is_timed_out = true;
					rollback_to = self.ctx.tick.id_consensus;

					if TRACE_TICK_ADVANCEMENT {
						debug!("client {} entered timeout state", id);
					}
				}
			}
		}

		//in an ideal world, input_history now contains, for
		//every client, inputs corresponding to tick id's
		//[consensus - 1, cur]. index 1 definitely refers
		//to tick.is_consensus, but the exact length of each
		//client's vec is unpredictable and depends on ping/
		//latency. it's even possible they may have too many.
		//it is possible that the rollback amount is 0 ticks,
		//in the case that the server is empty, hasn't
		//received any inputs, or the (unideal) scenario
		//that all clients' tick.id_cur is at/ahead of the
		//server's
		let rollback_amount = self.rollback(rollback_to);
		debug_assert_eq!(rollback_to, self.ctx.tick.id_cur);

		//this may trigger rollback to consensus
		self.trigger_net_events(rollback_amount);

		//tick.id_consensus is calculated based on
		//what is the oldest tick id for which an input
		//has been received from all clients
		let max_advance = tick_id_target - self.ctx.tick.id_consensus;
		let advance = self
			.input_history
			.values()
			.map(|history| {
				if history.is_timed_out {
					//don't wait for this client
					usize::MAX
				} else {
					history.entries.len() - 1
				}
			})
			.min()
			.map(|min| min as TickID)
			.unwrap_or(max_advance) //if there are no clients, insta-advance to tick_id_target
			.min(max_advance); //if all clients are living in the future, prevent them from fast forwarding the whole server

		debug_assert!(self.ctx.tick.id_cur == self.ctx.tick.id_consensus || advance == 0);
		self.ctx.tick.id_consensus += advance;

		self.simulate(tick_id_target);
	}

	fn trigger_net_events(&mut self, mut rollback_amount: TickID) {
		if self.net_events.is_empty() {
			return;
		}

		if TRACE_TICK_ADVANCEMENT {
			debug!("net events triggered");
		}

		//full rollback required
		rollback_amount += self.rollback(self.ctx.tick.id_consensus);

		//split this tick into 2 halves:
		//state changes caused by net events, and
		//state changes caused by the simulation
		self.ctx.diff.rollback_begin_tick(TickType::NetEvents);
		for &client in self.comms.keys() {
			let buffer = self.ctx.diff.tx_begin_tick(client, true).unwrap();
			TickType::NetEvents.ser_tx(buffer);
		}

		while let Some(event) = self.net_events.pop_front() {
			match event {
				UnrollbackableNetEvent::ServerStart => {
					(self.cb.on_server_start)(&mut self.ctx.state, self.ctx.diff.to_consensus());
				}

				UnrollbackableNetEvent::ClientConnect(comms) => {
					let new_client_id = self
						.ctx
						.state
						.clients
						.add_with_client_owned(ClientStateKind::Owned, &mut self.ctx.diff)
						.0;

					//game logic-specific
					(self.cb.on_client_connect)(
						&mut self.ctx.state,
						new_client_id,
						self.ctx.tick.id_cur,
						self.ctx.diff.to_consensus(),
					);

					let snapshot = snapshot_serdes::ser_new_client(
						&self.ctx.state,
						NewClientHeader {
							client_id: new_client_id,
							tick_id_snapshot: self.ctx.tick.id_cur,
							fast_forward_ticks: rollback_amount,
						},
					);

					#[allow(unused_must_use)]
					//safe to ignore error. if client has disconnected, sim thread will be notified shortly
					comms.to_client.send(SimToClientCommand::SendState(snapshot));

					self.comms.insert(new_client_id, comms);
					self.input_history
						.entry(new_client_id)
						.or_default()
						.generate_bogus_inputs(rollback_amount); //client is still responsible for sending an input for this tick
					let buffer = self.ctx.diff.on_connect(new_client_id);
					TickType::NetEvents.ser_tx(buffer);
				}

				UnrollbackableNetEvent::ClientDisconnect(old_client_id) => {
					self.ctx.diff.on_disconnect(old_client_id);
					self.input_history.remove(&old_client_id).unwrap();
					self.comms.remove(&old_client_id).unwrap();

					//game logic-specific
					(self.cb.on_client_disconnect)(
						&mut self.ctx.state,
						old_client_id,
						self.ctx.tick.id_cur,
						self.ctx.diff.to_consensus(),
					);

					self.ctx
						.state
						.clients
						.remove(old_client_id, &mut self.ctx.diff)
						.unwrap();
				}
			};
		}

		self.tx_all_clients();
	}

	pub(super) fn tx_all_clients(&mut self) {
		for (&client_id, comms) in self.comms.iter() {
			if let Some(diff) = self.ctx.diff.tx_end_tick(client_id) {
				#[allow(unused_must_use)]
				//safe to ignore error. if client has disconnected, sim thread will be notified shortly
				comms.to_client.send(SimToClientCommand::SendState(diff));
			}
		}
	}
}
