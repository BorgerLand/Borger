#![feature(vec_deque_truncate_front)] //https://github.com/rust-lang/rust/issues/140667

use crate::context::{GameContext, Immediate};
use crate::networked_types::primitive::usize32;
use crate::simulation_controller::SimControllerExternals;
use crate::simulation_state::{InputState, SimulationState};
use crate::tick::TickID;

#[cfg(feature = "server")]
use {crate::context::WaitForConsensus, crate::diff_ser::DiffSerializer};

#[cfg(feature = "client")]
use std::collections::VecDeque;

pub mod macros;
pub mod math;

///Drives the simulation tick; controls pretty
///much everything from atop its throne
pub mod simulation_controller;

///Defines bidirectional, RPC-like communication
///channels between threads and they data they
///can carry
pub mod thread_comms;

///Time-keeping
pub mod tick;

///Enforces correct nesting of multiplayer_tradeoff!
///using Rust's type system
pub mod context;

///Serdes strategies for all networked state types
pub mod networked_types;

#[cfg(feature = "client")]
///Allows TS code to efficiently read RS-owned
///memory by exposing sections of raw WASM memory
///as typed array buffers
pub mod js_bindings;

///Random small snippets of code that didn't seem
///to belong anywhere else
mod misc;
pub(crate) use misc::*;

mod handwritten {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod presentation_state;
	pub(crate) mod simulation_state;
	pub(crate) mod snapshot_serdes;
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod entities;
}

#[allow(non_camel_case_types)]
mod generated {
	pub(crate) mod constructors;
	pub(crate) mod diff_des;
	pub(crate) mod diff_ser;
	pub(crate) mod presentation_state;
	pub(crate) mod simulation_state;
	pub(crate) mod snapshot_serdes;
	pub(crate) mod untracked;

	#[cfg(feature = "client")]
	pub(crate) mod entities;
}

///Constructors for simulation state objects
pub(crate) mod constructors {
	pub use super::handwritten::constructors::*;
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

///Any changes to state during the simulation tick
///are recorded by this system as they're
///happening. Rollback and rx systems use this
///data to make multiplayer happen
pub(crate) mod diff_ser {
	pub use super::handwritten::diff_ser::*;

	#[cfg(feature = "client")]
	pub(crate) use super::generated::diff_ser::*;
}

///Stripped down version of simulation state (only
///fields marked with presentation: true), cloned
///and shipped to the presentation thread at the
///end of each client sided tick
pub mod presentation_state {
	pub use super::generated::presentation_state::*;
	pub use super::handwritten::presentation_state::*;
}

///Struct definitions of simulation state objects
pub mod simulation_state {
	pub use super::generated::simulation_state::*;
	pub use super::handwritten::simulation_state::*;
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

///Interpolation and presentation of entities
#[cfg(feature = "client")]
pub mod entities {
	pub use super::generated::entities::*;
	pub use super::handwritten::entities::*;
}

///Helpful types and macros when writing simulation logic
pub mod prelude {
	pub use crate::context::{
		AnyTradeoff, GameContext, Immediate, ImmediateOrWaitForServer, WaitForConsensus, WaitForServer,
	};

	pub use crate::diff_ser::DiffSerializer;
	pub use crate::multiplayer_tradeoff;
	pub use crate::simulation_state::*;
	pub use crate::tick::{TickID, TickInfo};
	pub use log::*;

	#[cfg(feature = "client")]
	pub use {crate::js_bindings::JSBindings, crate::presentation_state::PresentationTick};
}

pub struct SimulationCallbacks {
	//pipeline
	pub simulation_tick: fn(/*ctx*/ &mut GameContext<Immediate>),

	//input_ops
	pub input_validate: fn(/*sus*/ &mut InputState),
	pub input_predict_late: fn(
		/*last_known*/ &InputState,
		/*age*/ TickID,
		/*state*/ &SimulationState,
		/*client_id*/ usize32,
	) -> InputState,

	#[cfg(feature = "client")]
	pub input_merge: fn(/*combined*/ &mut InputState, /*new*/ &InputState),

	//net_events
	#[cfg(feature = "server")]
	pub on_server_start:
		fn(/*state*/ &mut SimulationState, /*diff*/ &mut DiffSerializer<WaitForConsensus>), //goes without saying tick id is 0
	#[cfg(feature = "server")]
	pub on_client_connect: fn(
		/*state*/ &mut SimulationState,
		/*client id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
	#[cfg(feature = "server")]
	pub on_client_disconnect: fn(
		/*state*/ &mut SimulationState,
		/*id*/ usize32,
		/*tick id*/ TickID,
		/*diff*/ &mut DiffSerializer<WaitForConsensus>,
	),
}

pub fn init(
	cb: SimulationCallbacks,

	#[cfg(feature = "client")] new_client_snapshot: VecDeque<u8>,
) -> SimControllerExternals {
	#[cfg(debug_assertions)]
	log::set_max_level(log::LevelFilter::Debug);

	simulation_controller::init(
		cb,
		#[cfg(feature = "client")]
		new_client_snapshot,
	)
}
