#[cfg(feature = "server")]
use {
	borger::simulation_controller::{self, SimThreading},
	clap::Parser,
	log::LevelFilter,
	simple_logger::SimpleLogger,
};

#[cfg(all(feature = "server", feature = "client"))]
compile_error!("Compiling both server+client into the same binary is dumb and broken.");
#[cfg(all(feature = "server", feature = "session_replay"))]
compile_error!("Feature flag `session_replay` in the server build is not yet implemented .");
#[cfg(all(feature = "server", feature = "singlethreaded"))]
compile_error!(
	"Feature flag `singlethreaded` in the server build is redundant. The server is inherently singlethreaded due to lack of a separate presentation thread."
);

#[cfg(feature = "server")]
pub mod flags;
#[cfg(feature = "server")]
pub mod net;

#[cfg(feature = "server")]
pub const SERVER_TITLE: &str = "Borger Game Server";
#[cfg(feature = "server")]
#[cfg(not(debug_assertions))]
const LOG_LEVEL: LevelFilter = LevelFilter::Info;
#[cfg(feature = "server")]
#[cfg(debug_assertions)]
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
	#[cfg(feature = "server")]
	{
		SimpleLogger::new().with_level(LOG_LEVEL).init().unwrap();

		let flags = flags::Flags::parse();
		let sim = simulation_controller::init_multithreaded(game::init());
		let SimThreading::Multithreaded(thread) = sim.internals;

		let sim_loop = tokio::task::spawn_blocking(move || thread.join().unwrap());
		let net_loop = tokio::spawn(net::init(sim.new_connection_sender, flags));

		//both of these are infinite loops and should never fail.
		//they are wrapped in tokio select in order to crash the
		//entire program if either actually does fail
		tokio::select! {
			_ = sim_loop => {}
			_ = net_loop => {}
		}

		std::process::exit(1);
	}
}
