#[cfg(feature = "server")]
use {clap::Parser, simple_logger::SimpleLogger};

#[cfg(feature = "server")]
pub mod flags;
#[cfg(feature = "server")]
pub mod net;

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
	#[cfg(feature = "server")]
	{
		SimpleLogger::new().init().unwrap();
		let flags = flags::Flags::parse();
		let sim = game_rs::simulation::init();
		net::init(sim.new_connection_sender, &flags).await;
	}
}
