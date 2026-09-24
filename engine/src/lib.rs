use crate::simulation::{Input, State};
use crate::simulation_controller::GameContext;
use borger_plugin_sdk::TickID;
use borger_plugin_sdk::diff_ser::DiffSerializer;
use borger_plugin_sdk::multiplayer_tradeoff::{Immediate, WaitForConsensus};
use borger_plugin_sdk::primitive::usize32;

pub(crate) mod multiplayer_tradeoff;
pub(crate) mod scope;

//things that really ought to be plugins but aren't
pub mod physics;
pub mod slotmap;

///Drives the simulation tick; controls pretty
///much everything from atop its throne
#[cfg_attr(not(any(feature = "server", feature = "client")), doc(hidden))]
pub mod simulation_controller;

///Defines bidirectional, RPC-like communication
///channels between threads and they data they
///can carry
#[cfg_attr(not(any(feature = "server", feature = "client")), doc(hidden))]
pub mod thread_comms;

///Time-keeping
pub(crate) mod tick;

mod handwritten {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod simulation;
	pub(crate) mod snapshot_serdes;
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod interpolation;
	#[cfg(feature = "client")]
	pub(crate) mod presentation;
}

#[allow(unused, dead_code)]
mod generated {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod simulation;
	pub(crate) mod snapshot_serdes;
	///Responsible for resetting fields with netVisibility: "Untracked"
	///back to their default values in between ticks. This is important
	///for maintaining deterministic behavior; otherwise, the data in
	///each untracked field is stale/left over from any arbitrary tick,
	///forward or backward in time. It'd be the same effect as reading
	///from uninitialized memory
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod interpolation;
	#[cfg(feature = "client")]
	pub(crate) mod presentation;

	#[cfg(any(feature = "server", feature = "client"))]
	pub(crate) mod plugin_exports;
}

///Struct definitions of state objects
#[cfg_attr(not(any(feature = "server", feature = "client")), doc(hidden))]
pub mod simulation {
	pub use super::generated::simulation::*;
	pub use super::handwritten::simulation::*;
}

///Any changes to state during the simulation tick
///are recorded by this system as they're
///happening. Rollback and rx systems use this
///data to make multiplayer happen
pub mod diff_ser {
	pub(crate) use super::handwritten::diff_ser::*;

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
///state. Used for sending server's current state
///to a newly connected client. Also used to predict
///the removal of a collection element. This is a
///complicated+expensive task because it requires
///creating a backup of the entire struct being
///removed in order to be able to roll back to
///before it was deleted
pub(crate) mod snapshot_serdes {
	pub use super::handwritten::snapshot_serdes::*;
}

///Stripped down version of state (only fields with
///with presentation enabled), cloned and shipped to
///the presentation thread at the end of each client
///sided tick
#[cfg(feature = "client")]
pub mod presentation {
	pub use super::generated::presentation::*;
	pub use super::handwritten::presentation::*;
}

///Interpolation and presentation of entities
#[cfg(feature = "client")]
pub mod interpolation {
	pub use super::generated::interpolation::*;
	pub use super::handwritten::interpolation::*;
}

///Helpful types and macros when writing simulation logic
pub mod prelude {
	pub use crate::SimulationInitOptions;
	pub use crate::simulation::*;
	pub use crate::simulation_controller::GameContext;
	pub use crate::tick::TickInfo;
	pub use borger_plugin_sdk::TickID;
	pub use borger_plugin_sdk::diff_ser::DiffSerializer;
	pub use borger_plugin_sdk::multiplayer_tradeoff::{
		AnyTradeOff, Immediate, ImmediateOrWaitForServer, WaitForConsensus, WaitForServer,
	};
	pub use borger_plugin_sdk::primitive::{isize32, usize32};
	pub use borger_plugin_sdk::traits::Interpolate;
	pub use borger_procmac::server;
	pub use log::{debug, error, info, warn};

	#[doc(inline)]
	pub use crate::multiplayer_tradeoff; //macro
}

pub struct SimulationInitOptions {
	//pipeline
	pub init_static_level_geom: Option<fn(/*state*/ &mut State)>,
	pub simulation_loop: fn(/*ctx*/ &mut GameContext<Immediate>),

	//input operations
	pub input_merge: fn(/*combined*/ &Input, /*new*/ &Input) -> Input,
	pub input_validate: fn(/*sus*/ &Input) -> Input,
	pub input_server_predict_late: fn(
		/*prv*/ &Input,
		/*state*/ &State,
		/*client_id*/ usize32,
		/*is_timed_out*/ bool,
	) -> Input,
	pub input_client_predict_late:
		fn(/*prv*/ &Input, /*state*/ &State, /*client_id*/ usize32) -> Input,

	//server_events
	pub on_server_start: fn(/*state*/ &mut State, /*diff*/ &mut DiffSerializer<WaitForConsensus>), //goes without saying tick id is 0
	pub on_client_connect: fn(
		/*state*/ &mut State,
		/*client id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
	pub on_client_disconnect: fn(
		/*state*/ &mut State,
		/*id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
}
