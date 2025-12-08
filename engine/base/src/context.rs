use std::mem;

use crate::diff_ser::DiffSerializer;
use crate::simulation_state::SimulationState;
use crate::tick::TickInfo;

pub struct GameContext<Tradeoff: AnyTradeoff> {
	pub state: SimulationState,
	pub tick: TickInfo,
	pub diff: DiffSerializer<Tradeoff>,
}

pub struct Immediate; //to immediate/server/consensus
pub struct WaitForServer; //to server/consensus
pub struct WaitForConsensus; //to consensus
#[derive(Default)]
pub(crate) struct Impl; //default contextless state used internally

pub trait AnyTradeoff {} //to consensus
impl AnyTradeoff for Immediate {}
impl AnyTradeoff for WaitForServer {}
impl AnyTradeoff for WaitForConsensus {}
impl AnyTradeoff for Impl {}

//convenience: useful if an entity may be controlled either
//by client or server (npc)
pub trait ImmediateOrWaitForServer: AnyTradeoff {} //to server/consensus
impl ImmediateOrWaitForServer for Immediate {}
impl ImmediateOrWaitForServer for WaitForServer {}

//transmutation between different AnyTradeoff is safe memory-wise
//because the struct layout is not influenced by it (used by
//phantom data only). however it is not safe multiplayer-wise and
//so the game itself should not be calling these directly

macro_rules! multiplayer_tradeoff_transitions {
	($type:ident) => {
		impl $type<Immediate> {
			#[doc(hidden)]
			pub unsafe fn _to_immediate(&mut self) -> &mut Self {
				self
			}
		}

		#[cfg(feature = "server")]
		impl<Tradeoff: ImmediateOrWaitForServer> $type<Tradeoff> {
			#[doc(hidden)]
			pub unsafe fn _to_server(&mut self) -> &mut $type<WaitForServer> {
				unsafe { mem::transmute(self) }
			}
		}

		#[cfg(feature = "server")]
		impl<Tradeoff: AnyTradeoff> $type<Tradeoff> {
			#[doc(hidden)]
			pub unsafe fn _to_consensus(&mut self) -> &mut $type<WaitForConsensus> {
				unsafe { mem::transmute(self) }
			}
		}
	};
}

multiplayer_tradeoff_transitions!(GameContext);
multiplayer_tradeoff_transitions!(DiffSerializer);

//internal only

impl GameContext<Impl> {
	pub(crate) fn to_immediate(&mut self) -> &mut GameContext<Immediate> {
		unsafe { mem::transmute(self) }
	}
}

#[cfg(feature = "server")]
impl DiffSerializer<Impl> {
	pub(crate) fn to_consensus(&mut self) -> &mut DiffSerializer<WaitForConsensus> {
		unsafe { mem::transmute(self) }
	}
}

impl<Tradeoff: AnyTradeoff> DiffSerializer<Tradeoff> {
	pub(crate) fn to_impl(&mut self) -> &mut DiffSerializer<Impl> {
		unsafe { mem::transmute(self) }
	}
}
