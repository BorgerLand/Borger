use crate::simulation::pipeline::simulation_tick;
use base::SimulationCallbacks;
use base::simulation_controller::SimControllerExternals;

#[cfg(feature = "server")]
use crate::simulation::net_events::*;

pub mod input;
pub mod pipeline;

#[cfg(feature = "server")]
pub mod net_events;

//custom game logic modules
pub mod character;
pub mod physics_test;

pub fn init(#[cfg(feature = "client")] new_client_snapshot: Vec<u8>) -> SimControllerExternals {
	base::init(
		SimulationCallbacks {
			simulation_tick,

			input_validate: input::validate,
			input_predict_late: input::predict_late,

			#[cfg(feature = "client")]
			input_merge: input::merge,

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
