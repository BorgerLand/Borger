use crate::simulation::{Client, ClientOwned, ClientRemote};
use borger_plugin_sdk::ClientKind;
use borger_plugin_sdk::primitive::usize32;
use borger_plugin_sdk::traits::ConstructCustomStruct;
use std::rc::Rc;

impl ConstructCustomStruct for Client {
	//note this the only time that the client_kind argument is used
	fn construct(path: &Rc<Vec<usize32>>, client_kind: ClientKind) -> Self {
		if client_kind == ClientKind::Owned {
			Client::Owned(ClientOwned::construct(path, ClientKind::Owned))
		} else {
			Client::Remote(ClientRemote::construct(path, ClientKind::Remote))
		}
	}
}
