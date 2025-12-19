use crate::simulation::pipeline::simulation_tick;
use base::SimulationCallbacks;
use base::simulation_controller::SimControllerExternals;

#[cfg(feature = "server")]
use crate::simulation::net_events::*;

#[cfg(feature = "client")]
use std::collections::VecDeque;

pub mod character;
pub mod input_ops;
pub mod pipeline;

#[cfg(feature = "server")]
pub mod net_events;

//custom game logic modules

pub fn init(#[cfg(feature = "client")] new_client_snapshot: VecDeque<u8>) -> SimControllerExternals {
	base::init(
		SimulationCallbacks {
			simulation_tick,

			input_validate: input_ops::validate,
			input_predict_late: input_ops::predict_late,

			#[cfg(feature = "client")]
			input_merge: input_ops::merge,

			#[cfg(feature = "server")]
			on_server_start,
			#[cfg(feature = "server")]
			on_client_connect,
			#[cfg(feature = "server")]
			on_client_disconnect,
		},
		#[cfg(feature = "client")]
		new_client_snapshot,
	)
}
