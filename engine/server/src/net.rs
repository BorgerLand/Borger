use crate::SERVER_TITLE;
use crate::flags::Flags;
use base::networked_types::primitive::usize_to_32;
use base::networked_types::primitive::usize32;
use base::thread_comms::{ClientToSimCommand, SimToClientCommand};
use log::info;
use std::array::TryFromSliceError;
use std::net::IpAddr;
use std::sync::mpsc::Sender as SyncSender;
use std::time::Duration;
use tokio::sync::mpsc::{
	self as async_mpsc, UnboundedReceiver as AsyncReceiver, UnboundedSender as AsyncSender,
};
use tokio::{fs::File, io::AsyncWriteExt};
use wtransport::endpoint::IncomingSession;
use wtransport::error::StreamWriteError;
use wtransport::tls::Sha256DigestFmt;
use wtransport::{Endpoint, Identity, RecvStream, SendStream, ServerConfig};

pub const NET_TIMEOUT: u64 = 10; //kill a connection after this many seconds of lag
pub const NET_INPUT_SIZE_LIMIT: usize32 = 512; //in bytes

pub async fn init(new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>, flags: &Flags) {
	let identity = match (&flags.fullchain, &flags.privkey) {
		(Some(fullchain), Some(privkey)) => {
			info!("Loaded HTTPS/TLS certificates");
			Identity::load_pemfiles(fullchain, privkey).await.unwrap()
		}
		(None, None) => Identity::self_signed(["localhost"]).unwrap(),
		_ => {
			panic!("--fullchain and --privkey must both be provided, or neither");
		}
	};

	let cert = identity.certificate_chain().as_slice()[0]
		.hash()
		.fmt(Sha256DigestFmt::BytesArray);

	if flags.devcert.is_some() {
		let devcert_path = flags.devcert.as_ref().unwrap().clone();
		tokio::spawn(async move {
			let mut file = File::create(devcert_path).await.unwrap();
			file.write_all(cert.as_bytes()).await.unwrap();
			file.flush().await.unwrap();
		});
	}

	let config = ServerConfig::builder()
		.with_bind_default(flags.port)
		.with_identity(identity)
		//both chromium+firefox have their own keepalive/idle settings that
		//wtransport respects. these are mainly for redundancy in the case
		//of misbehaving clients
		.keep_alive_interval(Some(Duration::from_secs(NET_TIMEOUT / 2))) //defaults to none/infinite
		.max_idle_timeout(Some(Duration::from_secs(NET_TIMEOUT)))
		.unwrap() //default 30
		.build();

	let server = Endpoint::server(config).unwrap();
	let actual_port = server.local_addr().unwrap().port();

	info!(
		"\x1b[102;30m{}, port {}{}: it's alive.\x1b[0m",
		SERVER_TITLE,
		actual_port,
		if flags.port == 0 { " (randomized)" } else { "" }
	);

	loop {
		let incoming_session = server.accept().await;
		tokio::spawn(listen_on_connect(incoming_session, new_connection_sender.clone()));
	}
}

async fn listen_on_connect(
	incoming_session: IncomingSession,
	new_connection_sender: SyncSender<AsyncSender<SimToClientCommand>>,
) {
	let session_request = incoming_session.await.unwrap();
	let ip = session_request.remote_address().ip();
	let client = session_request.accept().await.unwrap();
	let (state_stream, input_stream) = client.accept_bi().await.unwrap();

	let (to_client, mut from_sim) = async_mpsc::unbounded_channel();
	new_connection_sender.send(to_client).unwrap();

	let to_sim = match from_sim.recv().await {
		Some(SimToClientCommand::Connect(to_sim)) => to_sim,
		_ => unreachable!(),
	};

	info!("[{}] Client connected", ip);

	//js brainrot: this is like Promise.race
	//except loser promises are cancelled
	let err = tokio::select! {
		biased;

		//handle timeout or other general webtransport error
		general_err = client.closed() => general_err.to_string(),

		//receive commands from simulation thread in a loop
		sim_err = listen_for_sim(state_stream, from_sim) => sim_err,

		//receive input states in a loop
		input_stream_err = listen_for_input(input_stream, &to_sim) => input_stream_err,
	};

	listen_on_disconnect(ip, err, to_sim);
}

async fn listen_for_sim(
	mut state_stream: SendStream,
	mut from_sim: AsyncReceiver<SimToClientCommand>,
) -> String {
	loop {
		let state_result: Result<(), String> = async {
			match from_sim.recv().await {
				Some(SimToClientCommand::SendState(state_diff)) => async {
					state_stream
						.write_all(&usize_to_32(state_diff.len()).to_le_bytes())
						.await?;
					state_stream.write_all(&state_diff).await?;
					Ok(())
				}
				.await
				.map_err(|oops: StreamWriteError| oops.to_string()),

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

//receive input
async fn listen_for_input(mut input_stream: RecvStream, to_sim: &SyncSender<ClientToSimCommand>) -> String {
	loop {
		let input_result = async {
			//receive number of bytes in client's input state diff
			let input_size_bytes =
				&mut receive_packet(&mut input_stream, size_of::<usize32>() as usize32).await?;
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
			let input_bytes = receive_packet(&mut input_stream, input_size).await?;
			to_sim
				.send(ClientToSimCommand::ReceiveInput(input_bytes))
				.map_err(|oops| oops.to_string())?;

			Ok(())
		}
		.await;

		if input_result.is_err() {
			return input_result.unwrap_err().to_string();
		}
	}
}

fn listen_on_disconnect(ip: IpAddr, disconnect_reason: String, to_sim: SyncSender<ClientToSimCommand>) {
	#[allow(unused_must_use)] //shouldn't fail unless the simulation crashed too
	to_sim.send(ClientToSimCommand::Disconnect);

	info!("[{}] Client disconnected ({})", ip, disconnect_reason);
}

async fn receive_packet(stream: &mut RecvStream, size: usize32) -> Result<Vec<u8>, String> {
	let mut packet = vec![0_u8; size as usize];
	match stream.read_exact(&mut packet).await {
		Ok(()) => Ok(packet),
		Err(oops) => Err(oops.to_string()),
	}
}
