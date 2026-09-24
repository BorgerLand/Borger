use crate::diff_ser::DiffSerializer;
use std::mem;

pub struct Immediate; //to immediate/server/consensus
pub struct WaitForServer; //to server/consensus
pub struct WaitForConsensus; //to consensus

#[derive(Default)]
pub struct Impl; //default contextless state used internally

pub trait AnyTradeOff {} //to consensus
impl AnyTradeOff for Immediate {}
impl AnyTradeOff for WaitForServer {}
impl AnyTradeOff for WaitForConsensus {}
impl AnyTradeOff for Impl {}

//convenience: useful if an entity may be controlled either
//by client or server (npc)
pub trait ImmediateOrWaitForServer: AnyTradeOff {} //to server/consensus
impl ImmediateOrWaitForServer for Immediate {}
impl ImmediateOrWaitForServer for WaitForServer {}

//transmutation between different AnyTradeOff is safe memory-wise
//because the struct layout is not influenced by it (used by
//phantom data only). however it is not safe multiplayer-wise and
//so the game itself should not be calling these directly
#[macro_export]
macro_rules! multiplayer_tradeoff_transitions {
	($type:ident) => {
		#[cfg(feature = "server")]
		impl<TradeOff: ::borger_plugin_sdk::multiplayer_tradeoff::ImmediateOrWaitForServer> $type<TradeOff> {
			#[doc(hidden)]
			pub unsafe fn _to_server_unchecked(
				&mut self,
			) -> &mut $type<::borger_plugin_sdk::multiplayer_tradeoff::WaitForServer> {
				unsafe { mem::transmute(self) }
			}
		}

		#[cfg(feature = "server")]
		impl<TradeOff: ::borger_plugin_sdk::multiplayer_tradeoff::AnyTradeOff> $type<TradeOff> {
			#[doc(hidden)]
			pub unsafe fn _to_consensus_unchecked(
				&mut self,
			) -> &mut $type<::borger_plugin_sdk::multiplayer_tradeoff::WaitForConsensus> {
				unsafe { mem::transmute(self) }
			}
		}
	};
}

multiplayer_tradeoff_transitions!(DiffSerializer);

//using traits here to prevent the game from gaining
//access to these methods (can't import the traits
//without adding plugin sdk as a dependency)
#[cfg(feature = "server")]
pub trait DiffSerializerToConsensus {
	fn to_consensus(&mut self) -> &mut DiffSerializer<WaitForConsensus>;
}

#[cfg(feature = "server")]
impl DiffSerializerToConsensus for DiffSerializer<Impl> {
	fn to_consensus(&mut self) -> &mut DiffSerializer<WaitForConsensus> {
		unsafe { mem::transmute(self) }
	}
}

#[cfg_attr(not(any(feature = "server", feature = "client")), doc(hidden))]
pub trait DiffSerializerToImpl {
	fn to_impl(&mut self) -> &mut DiffSerializer<Impl>;
}

#[cfg_attr(not(any(feature = "server", feature = "client")), doc(hidden))]
impl<TradeOff: AnyTradeOff> DiffSerializerToImpl for DiffSerializer<TradeOff> {
	fn to_impl(&mut self) -> &mut DiffSerializer<Impl> {
		unsafe { mem::transmute(self) }
	}
}
