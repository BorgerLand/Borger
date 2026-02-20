#[cfg(feature = "server")]
use {clap::Parser, simple_logger::SimpleLogger};

#[cfg(feature = "server")]
pub mod flags;
#[cfg(feature = "server")]
pub mod net;

pub const SERVER_TITLE: &str = "Borger Game Server";

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
	#[cfg(feature = "server")]
	{
		SimpleLogger::new()
			.with_level(log::LevelFilter::Info)
			.init()
			.unwrap();

		let flags = flags::Flags::parse();
		let sim = game_rs::simulation::init();
		let sim_loop = tokio::task::spawn_blocking(move || sim.thread.join().unwrap());
		let net_loop = net::init(sim.new_connection_sender, &flags);

		//both of these are infinite loops and should never fail.
		//they are wrapped in tokio select in order to crash the
		//entire program if either actually does fail
		tokio::select! {
			_ = sim_loop => {}
			_ = net_loop => {}
		}
	}
}
