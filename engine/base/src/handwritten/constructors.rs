use crate::ClientStateKind;
#[cfg(feature = "server")]
use crate::NetVisibility;
use crate::networked_types::primitive::usize32;
use crate::simulation_state::{ClientState, ClientState_owned, ClientState_remote, InputState};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl InputState {
	#[wasm_bindgen(constructor)]
	pub fn new() -> InputState {
		InputState::default()
	}
}

//networked type constructors are not publicly exposed. the
//simulation controller owns all state and refuses to give up
//ownership. this is to prevent mistakes: it would allow
//overwriting structs in a way that the observering diff
//serializer can't track
//state.clients = different_clients_object; //no!!

//custom user-defined structs - required by collections in
//order to construct whatever values they hold
pub trait ConstructCustomStruct {
	fn construct(path: &Rc<Vec<usize32>>, client_kind: ClientStateKind) -> Self;
}

//built in collection and utility types
pub trait ConstructCollectionOrUtilityType {
	fn construct(
		path: &Rc<Vec<usize32>>,
		field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self;
}

impl ConstructCustomStruct for ClientState {
	//note this the only time that the client_kind argument is used
	fn construct(path: &Rc<Vec<usize32>>, client_kind: ClientStateKind) -> Self {
		if client_kind == ClientStateKind::Owned {
			ClientState::Owned(ClientState_owned::construct(path, ClientStateKind::Owned))
		} else {
			ClientState::Remote(ClientState_remote::construct(path, ClientStateKind::Remote))
		}
	}
}
