use borger_procmac::EnumSerDes;
pub use borger_procmac::diff_operation_enum;

pub mod diff_ser;
pub mod multiplayer_tradeoff;
pub mod traits;

///Primitive number types
//When receiving data, both server and client
//assume each other to be little endian. Otherwise block compilation
//(you'd get a broken build)
#[cfg(target_endian = "little")]
pub mod primitive;

extern crate self as borger_plugin_sdk; //needed by macro below
diff_operation_enum!("base");

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
pub enum ClientKind {
	//depending on state declaration, some of
	//these may not even be used
	NA, //as in n/a not applicable
	Owned,

	#[allow(dead_code)]
	Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumSerDes)]
#[repr(u8)]
pub enum TickType {
	///If server events are triggered, a "server events" tick is
	///actually only the first half of a complete tick.
	///Non-deterministic in nature
	ServerEvents,

	///Consensus tick is final. All inputs have been received
	///from all clients (or timeout occurred while waiting).
	///It will never be simulated again
	Consensus,

	///Predicted tick has not received inputs from all clients yet.
	///It is guaranteed to simulate again when either the late input
	///arrives or the laggy client disconnects
	Predicted,
}

//fun fact: tick id as u32 at a rate of 30hz gives a maximum of
//~4.5 years of gameplay before overflow. not good enough i say.
//the u64 loses some precision when casting to f64 later on but
//should still give a lot more than 4.5 years.
pub type TickID = u64;
