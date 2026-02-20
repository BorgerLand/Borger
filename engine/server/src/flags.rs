use crate::SERVER_TITLE;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = SERVER_TITLE)]
pub struct Flags {
	///Port number to run the WebTransport server on (0-65535)
	#[arg(long, default_value = "6969")]
	pub webtransport_port: u16,

	///Port number to run the WebSocket server on (0-65535)
	#[arg(long, default_value = "6996")]
	pub websocket_port: u16,

	/// /path/to/devcert.json. DO NOT USE IN PRODUCTION.
	///Required in development /to satisfy WebTransport's
	///TLS requirement. The server (net.rs) will generate
	///a self-signed certificate at the provided path upon
	///startup. The client (net.ts) ships this certificate
	///to the browser in order to allow it to connect.
	#[arg(long, value_name = "FILE")]
	pub devcert: Option<PathBuf>,

	/// /path/to/fullchain.pem. Required in production
	///to satisfy WebTransport's TLS requirement. This
	///file is typically obtained via Certbot and Let's
	///Encrypt:
	///https://certbot.eff.org/instructions?ws=other&os=snap
	///Development builds should use --devcert instead.
	#[arg(long, value_name = "FILE")]
	pub fullchain: Option<PathBuf>,

	/// /path/to/privkey.pem. Required in production
	///to satisfy WebTransport's TLS requirement. This
	///file is typically obtained via Certbot and Let's
	///Encrypt:
	///https://certbot.eff.org/instructions?ws=other&os=snap
	///Development builds should use --devcert instead.
	#[arg(long, value_name = "FILE")]
	pub privkey: Option<PathBuf>,
}
