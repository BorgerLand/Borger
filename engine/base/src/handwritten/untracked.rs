use crate::simulation_state::ClientState;

pub trait UntrackedState {
	fn reset_untracked(&mut self);
}

impl<T: Default> UntrackedState for T {
	fn reset_untracked(&mut self) {
		*self = Self::default();
	}
}

impl UntrackedState for ClientState {
	fn reset_untracked(&mut self) {
		match self {
			Self::Owned(client) => client.reset_untracked(),
			Self::Remote(client) => client.reset_untracked(),
		}
	}
}
