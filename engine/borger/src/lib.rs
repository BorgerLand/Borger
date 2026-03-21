use crate::diff_ser::DiffSerializer;
use crate::multiplayer_tradeoff::{Immediate, WaitForConsensus};
use crate::networked_types::primitive::usize32;
use crate::simulation_controller::GameContext;
use crate::simulation_state::{Input, SimulationState};
use crate::tick::TickID;

pub mod multiplayer_tradeoff;
pub mod physics;

///Drives the simulation tick; controls pretty
///much everything from atop its throne
pub mod simulation_controller;

///Defines bidirectional, RPC-like communication
///channels between threads and they data they
///can carry
pub mod thread_comms;

///Time-keeping
pub mod tick;

///Serdes strategies for all networked state types
pub mod networked_types;

///Random small snippets of code that didn't seem
///to belong anywhere else
mod misc;
pub(crate) use misc::*;

mod handwritten {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod interpolation;
	pub(crate) mod simulation_state;
	pub(crate) mod snapshot_serdes;
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod presentation;
}

mod generated {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod interpolation;
	pub(crate) mod simulation_state;
	pub(crate) mod snapshot_serdes;
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod presentation;
}

///Struct definitions of simulation state objects
pub mod simulation_state {
	pub use super::generated::simulation_state::*;
	pub use super::handwritten::simulation_state::*;
}

///Constructors for simulation state objects
pub(crate) mod constructors {
	pub use super::handwritten::constructors::*;
}

///Any changes to state during the simulation tick
///are recorded by this system as they're
///happening. Rollback and rx systems use this
///data to make multiplayer happen
pub(crate) mod diff_ser {
	pub use super::handwritten::diff_ser::*;

	#[cfg(feature = "client")]
	pub(crate) use super::generated::diff_ser::*;
}

///Parses and executes operations record by
///diff_ser. Rollback system undoes changes to
///predicted state in order to account for the rx
///system receiving new information (server
///receiving late inputs, client receiving
///authoritative state). rx system applies state
///changes that were received
pub(crate) mod diff_des {
	pub use super::handwritten::diff_des::*;

	#[cfg(feature = "server")]
	pub use super::generated::diff_des::*;
}

///Take a snapshot (ser/des) of all or part of the
///simulation state. Used for sending server's
///current state to a newly connected client. Also
///used to predict the removal of a collection
///element. This is a complicated+expensive task
///because it requires creating a backup of the
///entire struct being removed in order to be able
///to roll back to before it was deleted
pub(crate) mod snapshot_serdes {
	pub use super::handwritten::snapshot_serdes::*;
}

///Responsible for resetting fields with netVisibility: "Untracked"
///back to their default values in between ticks. This is important
///for maintaining deterministic behavior; otherwise, the data in
///each untracked field is stale/left over from any arbitrary tick,
///forward or backward in time. It'd be the same effect as reading
///from uninitialized memory
pub(crate) mod untracked {
	pub use super::handwritten::untracked::*;
}

///Stripped down version of simulation state (only
///fields with presentation enabled), cloned and
///shipped to the presentation thread at the end of
///each client sided tick
#[cfg(feature = "client")]
pub mod presentation {
	pub use super::generated::presentation::*;
	pub use super::handwritten::presentation::*;
}

///Interpolation and presentation of entities
pub mod interpolation {
	#[cfg(feature = "client")]
	pub use super::generated::interpolation::*;

	pub use super::handwritten::interpolation::*;
}

///Helpful types and macros when writing simulation logic
pub mod prelude {
	pub use crate::SimulationInitOptions;
	pub use crate::diff_ser::DiffSerializer;
	pub use crate::multiplayer_tradeoff; //macro
	pub use crate::multiplayer_tradeoff::*;
	pub use crate::networked_types::primitive::usize32;
	pub use crate::simulation_controller::GameContext;
	pub use crate::simulation_controller::{SimControllerExternals, init as init_simulation};
	pub use crate::simulation_state::*;
	pub use crate::tick::{TickID, TickInfo};
	pub use borger_procmac::server;
	pub use log::*;
}

pub struct SimulationInitOptions {
	//pipeline
	pub simulation_loop: fn(/*ctx*/ &mut GameContext<Immediate>),
	pub new_client_snapshot: Vec<u8>,

	//input operations
	pub input_merge: fn(/*combined*/ &Input, /*new*/ &Input) -> Input,
	pub input_validate: fn(/*sus*/ &Input) -> Input,
	pub input_predict_late: fn(
		/*prv*/ &Input,
		/*is_timed_out*/ bool,
		/*state*/ &SimulationState,
		/*client_id*/ usize32,
	) -> Input,

	//server_events
	pub on_server_start:
		fn(/*state*/ &mut SimulationState, /*diff*/ &mut DiffSerializer<WaitForConsensus>), //goes without saying tick id is 0
	pub on_client_connect: fn(
		/*state*/ &mut SimulationState,
		/*client id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
	pub on_client_disconnect: fn(
		/*state*/ &mut SimulationState,
		/*id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
}
