use crate::ClientStateGeneric;
use crate::networked_types::collections::slotmap::SlotMap;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::InputState;
use crate::simulation_state::{ClientState_owned, ClientState_remote};

pub type ClientState = ClientStateGeneric<ClientState_owned, ClientState_remote>;

pub fn get_owned_client(clients: &SlotMap<ClientState>, id: usize32) -> Option<&ClientState_owned> {
	clients.get(id)?.as_owned()
}

pub fn get_owned_client_mut(
	clients: &mut SlotMap<ClientState>,
	id: usize32,
) -> Option<&mut ClientState_owned> {
	clients.get_mut(id)?.as_owned_mut()
}

impl SlotMap<ClientState> {
	pub fn owned_clients(&self) -> impl Iterator<Item = &ClientState_owned> {
		self.values().filter_map(|client| match client {
			ClientState::Owned(owned) => Some(owned),
			_ => None,
		})
	}

	pub fn owned_clients_mut(&mut self) -> impl Iterator<Item = &mut ClientState_owned> {
		self.values_mut().filter_map(|client| match client {
			ClientState::Owned(owned) => Some(owned),
			_ => None,
		})
	}
}

//wraps input state in a separate struct to allow disjoint
//borrows from the client state
#[derive(Debug)]
pub struct InputStateHistory {
	pub(crate) cur: InputState,
	pub(crate) cur_predicted: bool,
	pub(crate) prv: InputState,
	pub(crate) prv_predicted: bool,
}

impl InputStateHistory {
	pub(crate) fn default() -> Self {
		Self {
			cur: InputState::default(),
			cur_predicted: false,
			prv: InputState::default(),
			prv_predicted: false,
		}
	}

	pub fn get(&self) -> &InputState {
		&self.cur
	}

	pub fn is_predicted(&self) -> bool {
		self.cur_predicted
	}

	pub fn get_prv(&self) -> &InputState {
		&self.prv
	}

	pub fn is_prv_predicted(&self) -> bool {
		self.prv_predicted
	}
}
