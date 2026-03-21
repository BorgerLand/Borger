use crate::ClientKind;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::{Client, ClientOwned, ClientRemote};
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

//networked type constructors are not publicly exposed. the
//simulation controller owns all state and refuses to give up
//ownership. this is to prevent mistakes: it would allow
//overwriting structs in a way that the observering diff
//serializer can't track
//state.clients = different_clients_object; //no!!

//custom user-defined structs - required by collections in
//order to construct whatever values they hold
pub trait ConstructCustomStruct {
	fn construct(path: &Rc<Vec<usize32>>, client_kind: ClientKind) -> Self;
}

//built in collection and utility types
pub trait ConstructCollectionOrUtilityType {
	fn construct(
		path: &Rc<Vec<usize32>>,
		field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self;
}

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
