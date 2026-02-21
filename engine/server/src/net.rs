use crate::SERVER_TITLE;
use crate::flags::Flags;
use base::networked_types::primitive::usize_to_32;
use base::networked_types::primitive::usize32;
use base::thread_comms::{ClientToSimCommand, SimToClientCommand};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use std::array::TryFromSliceError;
use std::io::Error as IOError;
use std::net::SocketAddr;
use std::sync::mpsc::Sender as SyncSender;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{
	self as async_mpsc, UnboundedReceiver as AsyncReceiver, UnboundedSender as AsyncSender,
};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use wtransport::endpoint::IncomingSession;
use wtransport::error::StreamWriteError;
use wtransport::tls::Sha256DigestFmt;
use wtransport::tls::rustls::ServerConfig as WSServerConfig;
use wtransport::tls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use wtransport::{Endpoint, Identity, RecvStream, SendStream, ServerConfig as WTServerConfig};

pub const NET_TIMEOUT: u64 = 10; //kill a connection after this many seconds of lag. should match net.ts
pub const NET_INPUT_SIZE_LIMIT: usize32 = 512; //in bytes

pub async fn init(new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>, flags: &Flags) {
	let external_certs = match (&flags.fullchain, &flags.privkey) {
		(Some(fullchain), Some(privkey)) => Some((fullchain, privkey)),
		(None, None) => None,
		_ => panic!("--fullchain and --privkey must both be provided, or neither"),
	};

	let wt_identity = match external_certs {
		Some((fullchain, privkey)) => Identity::load_pemfiles(fullchain, privkey).await.unwrap(),
		None => Identity::self_signed(["localhost"]).unwrap(),
	};

	let wt_devcert = wt_identity.certificate_chain().as_slice()[0]
		.hash()
		.fmt(Sha256DigestFmt::BytesArray);

	if flags.devcert.is_some() {
		let devcert_path = flags.devcert.clone().unwrap();
		tokio::spawn(async move { fs::write(devcert_path, wt_devcert.as_bytes()).await.unwrap() });
	}

	let wt_config = WTServerConfig::builder()
		.with_bind_default(flags.webtransport_port)
		.with_identity(wt_identity.clone_identity())
		//both chromium+firefox have their own keepalive/idle settings that
		//wtransport respects. these are mainly for redundancy in the case
		//of misbehaving clients
		.keep_alive_interval(Some(Duration::from_secs(NET_TIMEOUT / 2))) //defaults to none/infinite
		.max_idle_timeout(Some(Duration::from_secs(NET_TIMEOUT)))
		.unwrap() //default 30
		.build();

	let wt_server = Endpoint::server(wt_config).unwrap();
	let wt_actual_port = wt_server.local_addr().unwrap().port();

	let ws_server = TcpListener::bind(("::", flags.websocket_port)).await.unwrap();
	let ws_actual_port = ws_server.local_addr().unwrap().port();

	//siphon the certificates back out of the webtransport server
	//to reuse in websocket
	let ws_certs = TlsAcceptor::from(std::sync::Arc::new(
		WSServerConfig::builder()
			.with_no_client_auth()
			.with_single_cert(
				wt_identity
					.certificate_chain()
					.as_slice()
					.iter()
					.map(|cert| CertificateDer::from(cert.der().to_vec()))
					.collect(),
				PrivateKeyDer::try_from(wt_identity.private_key().secret_der().to_vec()).unwrap(),
			)
			.unwrap(),
	));

	info!(
		"\x1b[102;30m{}, WebTransport UDP port {}{}, WebSocket TCP port {}{}: it's alive.\x1b[0m",
		SERVER_TITLE,
		wt_actual_port,
		if flags.webtransport_port == 0 {
			" (randomized)"
		} else {
			""
		},
		ws_actual_port,
		if flags.websocket_port == 0 {
			" (randomized)"
		} else {
			""
		}
	);

	//run the 2 servers concurrently
	tokio::join!(
		async {
			loop {
				let incoming_session = wt_server.accept().await;
				tokio::spawn(wt_listen_on_connect(
					incoming_session,
					new_connection_sender.clone(),
				));
			}
		},
		async {
			loop {
				let incoming_session = ws_server.accept().await;
				tokio::spawn(ws_listen_on_connect(
					incoming_session,
					new_connection_sender.clone(),
					ws_certs.clone(),
				));
			}
		}
	);
}

//(to_sim, from_sim)
async fn inform_sim_of_client(
	new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
) -> (SyncSender<ClientToSimCommand>, AsyncReceiver<SimToClientCommand>) {
	let (to_client, mut from_sim) = async_mpsc::unbounded_channel();
	new_connection_sender.send(to_client).unwrap();

	let to_sim = match from_sim.recv().await {
		Some(SimToClientCommand::Connect(to_sim)) => to_sim,
		_ => unreachable!(),
	};

	(to_sim, from_sim)
}

trait StateStream {
	async fn send(&mut self, state_diff: Vec<u8>) -> Result<(), String>;
}

async fn listen_for_sim(
	mut state_stream: impl StateStream,
	mut from_sim: AsyncReceiver<SimToClientCommand>,
) -> String {
	loop {
		let state_result: Result<(), String> = async {
			match from_sim.recv().await {
				Some(SimToClientCommand::SendState(state_diff)) => state_stream.send(state_diff).await,

				Some(SimToClientCommand::RequestKick(reason)) => {
					Err("kicked from the simulation: ".to_string() + &reason)
				}

				Some(SimToClientCommand::Connect(_)) => unreachable!(),
				None => unreachable!(), //simulation crashed
			}
		}
		.await;

		if let Err(oops) = state_result {
			return oops;
		}
	}
}

//---WEBTRANSPORT---//

async fn wt_listen_on_connect(
	incoming_session: IncomingSession,
	new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
) {
	let session_request = incoming_session.await.unwrap();
	let ip = session_request.remote_address().ip();
	let client = session_request.accept().await.unwrap();
	let (state_stream, input_stream) = client.accept_bi().await.unwrap();

	let (to_sim, from_sim) = inform_sim_of_client(new_connection_sender).await;
	info!("[{}] WebTransport client connected", ip);

	//js brainrot: this is like Promise.race
	//except loser promises are cancelled
	let err = tokio::select! {
		biased;

		//handle timeout or other general webtransport error
		general_err = client.closed() => general_err.to_string(),

		//receive commands from simulation thread in a loop
		sim_err = listen_for_sim(WTStateStream(state_stream), from_sim) => sim_err,

		//receive input states in a loop
		input_stream_err = wt_listen_for_input(input_stream, &to_sim) => input_stream_err,
	};

	#[allow(unused_must_use)] //shouldn't fail unless the simulation crashed too
	to_sim.send(ClientToSimCommand::Disconnect);
	info!("[{}] WebTransport client disconnected ({})", ip, err);
}

struct WTStateStream(SendStream);
impl StateStream for WTStateStream {
	async fn send(&mut self, state_diff: Vec<u8>) -> Result<(), String> {
		async {
			self.0
				.write_all(&usize_to_32(state_diff.len()).to_le_bytes())
				.await?;
			self.0.write_all(&state_diff).await?;
			Ok(())
		}
		.await
		.map_err(|oops: StreamWriteError| oops.to_string())
	}
}

async fn wt_listen_for_input(
	mut input_stream: RecvStream,
	to_sim: &SyncSender<ClientToSimCommand>,
) -> String {
	loop {
		let input_result = async {
			//receive number of bytes in client's input state diff
			let input_size_bytes = timeout(
				Duration::from_secs(NET_TIMEOUT),
				wt_receive_packet(&mut input_stream, size_of::<usize32>() as usize32),
			)
			.await
			.map_err(|_| "input stream timed out".to_string())??;

			let input_size = usize32::from_le_bytes(
				input_size_bytes.as_slice()[..4]
					.try_into()
					.map_err(|oops: TryFromSliceError| oops.to_string())?,
			);

			//if client requests the server to allocate
			//too much memory, kick the client to
			//prevent an oom crash. default 0.5kb is
			//intentionally very small. you'd have to
			//be destroying your keyboard and declare
			//too many fields on the InputState struct
			//to breach that limit
			if input_size > NET_INPUT_SIZE_LIMIT {
				return Err(format!(
					"Received too large of an input state diff (got {}b, max {}b)",
					input_size, NET_INPUT_SIZE_LIMIT
				));
			}

			//receive client's input state diff
			let input_bytes = wt_receive_packet(&mut input_stream, input_size).await?;
			to_sim
				.send(ClientToSimCommand::ReceiveInput(input_bytes))
				.map_err(|oops| oops.to_string())?;

			Ok(())
		}
		.await;

		if let Err(input_error) = input_result {
			return input_error;
		}
	}
}

async fn wt_receive_packet(stream: &mut RecvStream, size: usize32) -> Result<Vec<u8>, String> {
	let mut packet = vec![0_u8; size as usize];
	match stream.read_exact(&mut packet).await {
		Ok(()) => Ok(packet),
		Err(oops) => Err(oops.to_string()),
	}
}

//---WEBSOCKET---//

async fn ws_listen_on_connect(
	incoming_session: Result<(TcpStream, SocketAddr), IOError>,
	new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
	certs: TlsAcceptor,
) {
	let incoming_session = incoming_session.unwrap();
	let ip = incoming_session.1.ip();
	let tls_stream = match certs.accept(incoming_session.0).await {
		Ok(tls_stream) => tls_stream,
		Err(oops) => {
			return error!(
				"[{}] WebSocket client reported invalid certificates (have you dismissed the browser warning?): {:?}",
				ip, oops
			);
		}
	};

	let mut buf_stream = BufReader::new(tls_stream);

	let is_ws = match buf_stream.fill_buf().await {
		Ok(peeked) => String::from_utf8_lossy(peeked).contains("Upgrade: websocket"),
		Err(_) => return,
	};

	//if visiting the page over plain https, most likely what happened
	//is a dev is manually clearing the certificate error and they
	//want to go back to the game now. try to auto redirect ("Your
	//browser may be blocking the WSS connection" error)
	if !is_ws {
		let len = buf_stream.buffer().len();
		buf_stream.consume(len);
		let content = "<script>history.back()</script>";
		buf_stream
			.write_all(
				format!(
					"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
					content.len(),
					content
				)
				.as_bytes(),
			)
			.await
			.unwrap();

		//weirdly unidiomatic api to not consume buf_stream after shutdown?
		buf_stream.shutdown().await.unwrap();
		return;
	}

	let ws_stream = tokio_tungstenite::accept_async(buf_stream).await.unwrap();
	let (state_stream, input_stream) = ws_stream.split();

	let (to_sim, from_sim) = inform_sim_of_client(new_connection_sender).await;
	info!("[{}] WebSocket client connected", ip);

	let err = tokio::select! {
		biased;

		//receive commands from simulation thread in a loop
		sim_err = listen_for_sim(WSStateStream(state_stream), from_sim) => sim_err,

		//receive input states in a loop
		input_stream_err = ws_listen_for_input(input_stream, &to_sim) => input_stream_err,
	};

	#[allow(unused_must_use)] //shouldn't fail unless the simulation crashed too
	to_sim.send(ClientToSimCommand::Disconnect);
	info!("[{}] WebSocket client disconnected ({})", ip, err);
}

struct WSStateStream(SplitSink<WebSocketStream<BufReader<TlsStream<TcpStream>>>, Message>);
impl StateStream for WSStateStream {
	async fn send(&mut self, state_diff: Vec<u8>) -> Result<(), String> {
		self.0
			.send(Message::Binary(state_diff.into()))
			.await
			.map_err(|oops| oops.to_string())
	}
}

async fn ws_listen_for_input(
	mut input_stream: SplitStream<WebSocketStream<BufReader<TlsStream<TcpStream>>>>,
	to_sim: &SyncSender<ClientToSimCommand>,
) -> String {
	loop {
		match timeout(Duration::from_secs(NET_TIMEOUT), input_stream.next()).await {
			Err(_) => break "input stream timed out".to_string(),
			Ok(Some(Ok(Message::Binary(input_bytes)))) => {
				if let Err(e) = to_sim.send(ClientToSimCommand::ReceiveInput(input_bytes.into())) {
					break e.to_string();
				}
			}
			Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
				break "connection locally closed".to_string();
			}
			Ok(Some(Ok(Message::Ping(_)))) | Ok(Some(Ok(Message::Pong(_)))) => continue,
			Ok(Some(Ok(Message::Text(_)))) | Ok(Some(Ok(Message::Frame(_)))) => {
				break "received corrupt data".to_string();
			}
			Ok(Some(Err(e))) => {
				break e.to_string();
			}
		}
	}
}
