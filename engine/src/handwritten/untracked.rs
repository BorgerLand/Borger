use crate::simulation::Client;
use borger_plugin_sdk::traits::UntrackedState;

impl UntrackedState for Client {
	fn reset_untracked(&mut self) {
		match self {
			Self::Owned(client) => client.reset_untracked(),
			Self::Remote(client) => client.reset_untracked(),
		}
	}
}
