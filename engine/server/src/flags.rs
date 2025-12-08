use clap::Parser;
use std::path::PathBuf;

pub const SERVER_DESCRIPTION: &str = "Borger Game Server";

#[derive(Parser, Debug)]
#[command(about = SERVER_DESCRIPTION)]
pub struct Flags {
	///Port number to run the server on (0-65535)
	#[arg(short, long, default_value = "6969")]
	pub port: u16,

	///path/to/devcert.json. Required in development
	///to satisfy WebTransport's TLS requirement. The
	///server (net.rs) will generate a self-signed
	///certificate/at the provided path upon startup.
	///The client (Net.ts) ships this certificate to
	///the browser in order to allow it to connect
	#[arg(long, value_name = "FILE")]
	pub devcert: Option<PathBuf>,
	//need args for privkey.pem/fullchain.pem
}
