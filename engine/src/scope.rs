#[derive(Debug, Clone)]
#[repr(C, u8)]
pub enum Scope<O, R> {
	//owned:
	//- server can access everything. it owns all client objects
	//- client can access public and server-client fields. it only owns 1 client object
	Owned(O) = 0,

	//remote:
	//- server will never have any remote client objects
	//- client can only access public fields
	Remote(R) = 1,
}

impl<O, R> Scope<O, R> {
	pub fn as_owned(&self) -> Option<&O> {
		match self {
			Self::Owned(client) => Some(client),
			_ => None,
		}
	}

	pub fn as_owned_mut(&mut self) -> Option<&mut O> {
		match self {
			Self::Owned(client) => Some(client),
			_ => None,
		}
	}

	pub fn as_remote(&self) -> Option<&R> {
		match self {
			Self::Remote(client) => Some(client),
			_ => None,
		}
	}

	pub fn as_remote_mut(&mut self) -> Option<&mut R> {
		match self {
			Self::Remote(client) => Some(client),
			_ => None,
		}
	}
}
