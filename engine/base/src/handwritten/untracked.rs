use crate::simulation_state::ClientState;

pub trait UntrackedState {
	fn reset_untracked(&mut self);
}

impl UntrackedState for ClientState {
	fn reset_untracked(&mut self) {
		match self {
			Self::Owned(client) => client.reset_untracked(),
			Self::Remote(client) => client.reset_untracked(),
		}
	}
}
