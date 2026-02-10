use crate::SERVER_TITLE;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = SERVER_TITLE)]
pub struct Flags {
	///Port number to run the server on (0-65535)
	#[arg(short, long, default_value = "6969")]
	pub port: u16,

	/// /path/to/devcert.json. Required in development
	///to satisfy WebTransport's TLS requirement. The
	///server (net.rs) will generate a self-signed
	///certificate at the provided path upon startup.
	///The client (Net.ts) ships this certificate to
	///the browser in order to allow it to connect. Do
	///not use in production.
	#[arg(long, value_name = "FILE")]
	pub devcert: Option<PathBuf>,

	/// /path/to/fullchain.pem. Required in production
	///to satisfy WebTransport's TLS requirement. This
	///file is typically obtained via Certbot and Let's
	///Encrypt:
	///
	///https://certbot.eff.org/instructions?ws=other&os=snap
	///
	///Development builds should use --devcert instead.
	#[arg(long, value_name = "FILE")]
	pub fullchain: Option<PathBuf>,

	/// /path/to/privkey.pem. Required in production
	///to satisfy WebTransport's TLS requirement. This
	///file is typically obtained via Certbot and Let's
	///Encrypt:
	///
	///https://certbot.eff.org/instructions?ws=other&os=snap
	///
	///Development builds should use --devcert instead.
	#[arg(long, value_name = "FILE")]
	pub privkey: Option<PathBuf>,
}
