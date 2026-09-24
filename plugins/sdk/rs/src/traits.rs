use crate::ClientKind;
use crate::primitive::{DeserializeOopsy, usize32};
use std::fmt::Debug;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

#[cfg(feature = "client")]
use {crate::TickID, crate::diff_ser::DiffSerializer, crate::multiplayer_tradeoff::Impl, std::vec};

#[allow(private_bounds)]
pub trait CustomStruct:
	ConstructCustomStruct + DiffDeserializeCustomStruct + SnapshotState + UntrackedState + Debug + 'static
{
}

impl<T> CustomStruct for T where
	T: ConstructCustomStruct + DiffDeserializeCustomStruct + SnapshotState + UntrackedState + Debug + 'static
{
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
	fn construct(path: &Rc<Vec<usize32>>, client_kind: ClientKind) -> Self;
}

pub trait ConstructPlugin {
	fn construct(
		path: &Rc<Vec<usize32>>,
		field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self;
}

pub trait DiffDeserializeCustomStruct {
	fn set_field_rollback(&mut self, field_id: usize32, buffer: &mut Vec<u8>)
	-> Result<(), DeserializeOopsy>;

	#[cfg(feature = "client")]
	fn set_field_rx(
		&mut self,
		field_id: usize32,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy>;

	//collections+plugins
	fn get_plugin(&mut self, field_id: usize32) -> Result<&mut dyn DiffDeserializePlugin, DeserializeOopsy>;
}

pub trait DiffDeserializePlugin {
	fn navigate_down(&mut self, element_id: usize32) -> Option<&mut dyn DiffDeserializeCustomStruct>;

	//note for both deserializers: diff_op has not been checked whether it
	//belongs to this type, so make sure to always begin with the impl with
	//match DiffOperation::try_from

	fn des_rollback(&mut self, diff_op: u8, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;

	#[cfg(feature = "client")]
	fn des_rx(
		&mut self,
		diff_op: u8,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy>;
}

pub trait SnapshotState {
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, client_id: usize32, buffer: &mut Vec<u8>);
	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		client_id: usize32,
		buffer: &mut impl Iterator<Item = u8>,
	) -> Result<(), DeserializeOopsy>;

	fn ser_rollback_predict_remove(&self, buffer: &mut Vec<u8>); //<-- this one needs to be rewritten in "reverse" compared to other 3
	fn des_rollback_predict_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;
}

pub trait UntrackedState {
	fn reset_untracked(&mut self);
}

impl<T: Default> UntrackedState for T {
	fn reset_untracked(&mut self) {
		*self = Self::default();
	}
}

#[cfg(feature = "client")]
pub trait PresentTick {
	type PresentationOutput: InterpolateTicks;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationOutput;
}

///Classic lerp/slerp helper for various simple math primitives
pub trait Interpolate: Copy {
	fn interpolate(prv: Self, cur: Self, amount: f32) -> Self;
}

//trait exists to fire events when some change occurs between
//ticks (eg. for slotmaps, adding+removing slots). prv has to
//be an option in order to fire initial collection add/remove
//events, otherwise no change would be detected
#[cfg(feature = "client")]
pub trait InterpolateTicks<Prv = Self> {
	type InterpolationOutput;
	fn interpolate_and_diff(
		prv: Option<&Prv>,
		cur: &Self,
		amount: f32,
		received_new_tick: bool,
	) -> Self::InterpolationOutput;
}
