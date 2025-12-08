use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::fmt::{Debug, Error, Formatter};
use std::time::Duration;
use web_time::Instant;

#[cfg(feature = "server")]
use {crate::networked_types::primitive::usize32, crate::thread_comms::SimToClientChannel};

//fun fact: tick id as u32 at a rate of 30hz gives a maximum of
//~4.5 years of gameplay before overflow. not good enough i say.
//the u64 loses some precision when casting to f64 later on but
//should still give a lot more than 4.5 years.
pub type TickID = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum TickType {
	///If net events are triggered, a "net events" tick is actually
	///only the first half of a complete tick. Non-deterministic in
	///nature
	NetEvents,

	///Unrollbackable tick is final. All inputs have been received
	///from all clients. It will never be simulated again
	Unrollbackable,

	///Predicted tick has not received inputs from all clients yet.
	///It is guaranteed to simulate again when either the late input
	///arrives or the laggy client disconnects
	Predicted,
}

pub struct TickInfo {
	//this value is not network synchronized in any way and so
	//isn't deterministic. it only measures how long the local
	//simulation has been running
	first: Instant,

	//all of these id's are incremental

	//rollbackable: oldest tick that can still be rolled back and
	//(as in, rewind simulation state to right before this tick
	//happened). controls the amount of state history that must be
	//stored for rollback. will only ever increase
	pub(crate) id_unrollbackable: TickID,

	//another way of looking at this number is "how many ticks have
	//completed start to finish" or "tick.id_unfinished". may increase
	//or decrease due to local rollbacks, causing old ticks to be
	//resimulated
	pub(crate) id_cur: TickID,
	//id_unrollbackable <= id_cur
	//the wider the gap between id's, the worse the performance
	//due to more rollbacks and retransmitting old ticks. this
	//also means that the laggiest client will hurt performance
	//for everyone, including the server
}

impl TickInfo {
	//simulation delta time/tick rate, in seconds/tick.
	//can be higher or lower than vsync refresh rate.
	//too low feels kinda floaty, too high hurts performance
	pub const SIM_DT: f32 = 1.0 / 30.0;

	pub(crate) fn new(id_start: TickID, fast_forward_ticks: TickID) -> Self {
		TickInfo {
			first: Instant::now()
				- Duration::from_secs_f64((id_start + fast_forward_ticks) as f64 * Self::SIM_DT as f64),
			id_unrollbackable: id_start,
			id_cur: id_start,
		}
	}

	pub fn id(&self) -> TickID {
		self.id_cur
	}

	//if true, this tick is being simulated for the final time.
	//non-deterministic code is allowed, and large transition events
	//(objective complete, game end, etc.) are encouraged to happen now
	#[cfg(feature = "server")]
	pub fn is_unrollbackable(&self) -> bool {
		//seems counterintuitive that unrollbackable
		//can be higher than cur, but this is because
		//id_unrollbackable is incremented at the start
		//of a tick while id_cur is incremented at the
		//end
		self.id_unrollbackable > self.id_cur
	}

	pub(crate) fn get_elapsed_at(id: TickID) -> Duration {
		Duration::from_secs_f64(Self::SIM_DT as f64 * id as f64)
	}

	pub(crate) fn get_instant(&self, id: TickID) -> Instant {
		self.first + Self::get_elapsed_at(id)
	}

	pub(crate) fn get_now(&self) -> Instant {
		self.get_instant(self.id_cur)
	}
}

impl Debug for TickInfo {
	fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
		#[derive(Debug)]
		#[allow(dead_code)]
		struct TickInfo {
			id_unrollbackable: TickID,
			id_cur: TickID,
		}

		Debug::fmt(
			&TickInfo {
				id_unrollbackable: self.id_unrollbackable,
				id_cur: self.id_cur,
			},
			f,
		)
	}
}

//triggering an unrollbackable game logic event is
//normally delayed because they're caused by something
//happening on a specific tick, so that tick needs
//to finalize/become unrollbackable before triggering
//the unrollbackable event. unrollbackable network
//events on the other hand do not care about happening
//on a specific tick, so in order to trigger them asap,
//server rolls back as far as it can to the most recent
//unrollbackable point in history
#[cfg(feature = "server")]
pub(crate) enum UnrollbackableNetEvent {
	ServerStart,
	ClientConnect(SimToClientChannel),
	//Chat, //required for executing game logic based on chat (/commands)
	ClientDisconnect(usize32),
}
