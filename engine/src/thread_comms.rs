use std::sync::mpsc::{Receiver as SyncReceiver, Sender as SyncSender};

#[cfg(feature = "server")]
use tokio::sync::mpsc::UnboundedSender as AsyncSender;

#[cfg(feature = "client")]
use crate::simulation_state::Input;

#[cfg(feature = "session_replay")]
use serde::{Deserialize, Serialize};

//server-sided code for communicating between
//a client event loop on the wtransport thread
//and the simulation thread
#[cfg(feature = "server")]
pub enum ClientToSimCommand {
	ReceiveInput(Vec<u8>), //received input from a client
	Disconnect,
}

#[cfg(feature = "server")]
#[derive(Debug)]
pub enum SimToClientCommand {
	Connect(SyncSender<ClientToSimCommand>), //new client connected
	SendState(Vec<u8>),
	RequestKick(String),
}

#[cfg(feature = "server")]
pub struct SimToClientChannel {
	pub to_client: AsyncSender<SimToClientCommand>,
	pub from_client: SyncReceiver<ClientToSimCommand>,
}

//client-sided code for communicating between
//main/presentation thread and simulation thread
#[cfg(feature = "client")]
#[cfg_attr(feature = "session_replay", derive(Deserialize, Serialize, Clone))]
pub enum PresentationToSimCommand {
	RawInput(Input),       //presentation thread sends hot fresh inputs here
	ReceiveState(Vec<u8>), //received state from the server
	Abort,
}

#[cfg(feature = "client")]
pub enum SimToPresentationCommand {
	//send diffs of merged inputs back to the main/
	//presentation thread in order to send over the
	//wire because the webtransport object lives there
	InputDiff(Vec<u8>),

	#[cfg(feature = "session_replay")]
	SessionReplayAction(SessionReplayAction),
}

#[cfg(feature = "session_replay")]
#[derive(Serialize, Deserialize)]
pub enum SessionReplayAction {
	Init(Vec<u8>),
	ScheduledTick,
	ReceiveComm(PresentationToSimCommand),
}

#[cfg(feature = "client")]
pub struct PresentationToSimChannel {
	pub to_sim: SyncSender<PresentationToSimCommand>,
	pub from_sim: SyncReceiver<SimToPresentationCommand>,
}

#[cfg(feature = "client")]
pub struct SimToPresentationChannel {
	pub to_presentation: SyncSender<SimToPresentationCommand>,
	pub from_presentation: SyncReceiver<PresentationToSimCommand>,
}
