use crate::Scope;
use crate::simulation_state::Input;
use crate::simulation_state::{ClientOwned, ClientRemote};

pub type Client = Scope<ClientOwned, ClientRemote>;

//wraps input state in a separate struct to allow disjoint
//borrows from the client state
#[derive(Debug)]
pub struct InputHistory {
	pub(crate) cur: InputHistoryEntry,
	pub(crate) prv: InputHistoryEntry,
}

impl InputHistory {
	pub(crate) fn default() -> Self {
		Self {
			cur: InputHistoryEntry::default(),
			prv: InputHistoryEntry::default(),
		}
	}

	pub fn get(&self) -> &InputHistoryEntry {
		&self.cur
	}

	pub fn get_prv(&self) -> &InputHistoryEntry {
		&self.prv
	}
}

#[derive(Default, Debug)]
pub struct InputHistoryEntry {
	pub state: Input,
	pub(crate) age: InputAge,
}

impl InputHistoryEntry {
	pub fn is_predicted(&self) -> bool {
		self.age == InputAge::Predicted
	}
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum InputAge {
	///This is the first time that this client's input has
	///run through the current simulation tick ID. For each
	///simulated tick, each client guarantees that its
	///current input state will be fresh exactly 1 or 0 times
	///(0 in the case that the client disconnects before the
	///server either acknowledges their input or times them
	///out)
	#[default]
	Fresh,

	///This is NOT the first time this client's input has
	///run through the current simulation tick ID. This tick
	///ID is being resimulated due to another client's inputs
	///arriving (or timing out)
	Resimulating,

	///This client's inputs have not arrived yet for the
	///current simulation tick ID. The state was predicted by
	///the server using input::predict_late. A client will
	///never see InputAge::Predicted.
	Predicted,
}
