use crate::DeserializeOopsy;
use crate::networked_types::primitive::PrimitiveSerDes;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::ClientState;
use crate::simulation_state::SimulationState;
use crate::tick::TickID;

pub trait SnapshotState {
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, client_id: usize32, buffer: &mut Vec<u8>);
	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		client_id: usize32,
		buffer: &mut impl Iterator<Item = u8>,
	) -> Result<(), DeserializeOopsy>;

	fn ser_rollback_predict_remove(&self, buffer: &mut Vec<u8>); //<-- this one needs to be rewritten in "reverse" compared to other 3
	fn des_rollback_predict_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;
}

impl SnapshotState for ClientState {
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, client_id: usize32, buffer: &mut Vec<u8>) {
		//since the server has no remote clients,
		//it's also safe not to impl ser_tx_new_client
		//for remote clients
		self.as_owned().unwrap().ser_tx_new_client(client_id, buffer);
	}

	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		client_id: usize32,
		buffer: &mut impl Iterator<Item = u8>,
	) -> Result<(), DeserializeOopsy> {
		match self {
			ClientState::Owned(client) => client.des_rx_new_client(client_id, buffer),
			ClientState::Remote(client) => client.des_rx_new_client(client_id, buffer),
		}
	}

	fn ser_rollback_predict_remove(&self, _: &mut Vec<u8>) {
		panic!("Attempted to predict the removal of a client. Ya just can't do that, son.");
	}

	fn des_rollback_predict_remove(&mut self, _: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		unreachable!();
	}
}

pub struct NewClientHeader {
	pub client_id: usize32,
	pub tick_id_snapshot: TickID,
	pub fast_forward_ticks: TickID,
}

#[cfg(feature = "server")]
pub fn ser_new_client(state: &SimulationState, header: NewClientHeader) -> Vec<u8> {
	let mut buffer_owned = Vec::new();
	let buffer = &mut buffer_owned;

	header.client_id.ser_tx(buffer);
	header.tick_id_snapshot.ser_tx(buffer);
	header.fast_forward_ticks.ser_tx(buffer);
	state.ser_tx_new_client(header.client_id, buffer);

	buffer_owned
}

//returns local client id
#[cfg(feature = "client")]
pub fn des_new_client(
	state: &mut SimulationState,
	buffer: Vec<u8>,
) -> Result<NewClientHeader, DeserializeOopsy> {
	let buffer = &mut buffer.into_iter();

	let header = NewClientHeader {
		client_id: PrimitiveSerDes::des_rx(buffer)?,
		tick_id_snapshot: PrimitiveSerDes::des_rx(buffer)?,
		fast_forward_ticks: PrimitiveSerDes::des_rx(buffer)?,
	};

	state.des_rx_new_client(header.client_id, buffer)?;

	Ok(header)
}
