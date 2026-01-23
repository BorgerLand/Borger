use crate::diff_des::DiffDeserializeState;
use crate::presentation_state::CloneToPresentationState;
use crate::snapshot_serdes::SnapshotState;
use crate::{constructors::ConstructCustomStruct, untracked::UntrackedState};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::fmt::Debug;

//alias for all state tracker subsystem traits
#[allow(private_bounds)]
pub trait NetState:
	CloneToPresentationState
	+ ConstructCustomStruct
	+ DiffDeserializeState
	+ SnapshotState
	+ UntrackedState
	+ Debug
	+ 'static
{
}

impl<T> NetState for T where
	T: CloneToPresentationState
		+ ConstructCustomStruct
		+ DiffDeserializeState
		+ SnapshotState
		+ UntrackedState
		+ Debug
		+ 'static
{
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetVisibility {
	//depending on state declaration, some of
	//these may not even be used
	#[allow(dead_code)]
	Private,

	#[allow(dead_code)]
	Owner,

	Public,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClientStateKind {
	//depending on state declaration, some of
	//these may not even be used
	NA, //as in n/a not applicable
	Owned,

	#[allow(dead_code)]
	Remote,
}

//generic client state reused by both
//simulation+presentation
#[derive(Debug)]
pub enum ClientStateGeneric<O, R> {
	//owned:
	//- server can access everything. it owns all client objects
	//- client can access public and server-client fields. it only owns 1 client object
	Owned(O),

	//remote:
	//- server will never have any remote client objects
	//- client can only access public fields
	Remote(R),
}

impl<O, R> ClientStateGeneric<O, R> {
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

#[derive(Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u8)]
pub enum DiffOperation {
	//state change
	TrackPrimitive,
	TrackSlotMapAdd,
	TrackSlotMapRemove,
	TrackSlotMapClear,

	//path navigation
	NavigateUp,    //unix analogy: "cd ../../.." go to parent directory
	NavigateDown,  //unix analogy: "cd x/y/z" open directory
	NavigateReset, //unix analogy: "cd /" go to root directory

	//(rollback only) insert a wall between ticks in
	//order to know when to stop rolling back a tick.
	//technically doable for tx system too but it's
	//simpler to just send the packet's size in bytes
	RollbackTickSeparator,
}

#[derive(Debug)]
pub enum DeserializeOopsy {
	BufferUnderflow,
	CorruptBool,
	CorruptDiffOperation,
	CorruptTickType,
	CorruptChar,
	CorruptVarInt,
	ObeseVarInt,
	PathNotFound,
	FieldNotFound,
}
