use crate::constructors::ConstructCollectionOrUtilityType;
use crate::diff_des::DiffDeserializeState;
use crate::diff_ser::DiffSerializer;
use crate::multiplayer_tradeoff::AnyTradeoff;
use crate::networked_types::primitive::{PrimitiveSerDes, usize32};
use crate::snapshot_serdes::SnapshotState;
use crate::untracked::UntrackedState;
use crate::{ClientKind, DeserializeOopsy, DiffOperation, TrackedState};
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

#[cfg(feature = "client")]
use {
	crate::interpolation::InterpolateTicks, crate::multiplayer_tradeoff::Impl,
	crate::presentation::PresentTick, crate::simulation_state::Client, crate::tick::TickID, std::any::TypeId,
	std::mem::MaybeUninit, std::ptr, std::vec,
};

//like a hashmap, except the key is an internally generated, unique, numeric id.
//currently does not do any sort of generational tracking
//guarantees:
//- values are stored in contiguous memory for fast iteration
//- slot id's are only reused upon wrapping around back to 0 due to crazy amount of add+remove cycles
//- get+remove (random accesses) are O(1)
//- add is amortized O(1), but O(n) if internal data structures need to resize
//- iteration order is deterministic but not in order of insertion*
//- 	*it will only be in order of insertion so long as elements are not removed from the "middle"
//- 	*you can grab the most recently inserted slot with .iter().last(), if no slots have been removed since inserting it

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSlotMap<V> {
	slots: Vec<(usize32, V)>,
	random_access: HashMap<usize32, usize32>, //<slot id, physical index>
	next_id: usize32,
}

impl<V> RawSlotMap<V> {
	pub fn new() -> Self {
		Self {
			slots: Vec::new(),
			random_access: HashMap::new(),
			next_id: 0,
		}
	}

	pub fn add(&mut self, value: V) -> (usize32, &mut V) {
		self.add_with_id(|_| value)
	}

	pub fn add_with_id(&mut self, value: impl FnOnce(usize32) -> V) -> (usize32, &mut V) {
		let start_id = self.next_id;
		let id = loop {
			let new_id = self.next_id;

			self.next_id = self.next_id.wrapping_add(1);
			if self.next_id == start_id {
				panic!("SlotMap's belly is too full to eat no more");
			}

			if !self.random_access.contains_key(&new_id) {
				break new_id;
			}
		};

		let physical_index = self.len();

		self.slots.push((id, value(id)));
		self.random_access.insert(id, physical_index);

		let slot = self.slots.last_mut().unwrap();
		(slot.0, &mut slot.1)
	}

	pub fn remove(&mut self, id: usize32) -> Option<V> {
		let physical_index_u32 = self.random_access.remove(&id)?;
		let physical_index_usize = physical_index_u32 as usize;
		let (_, removed_slot) = self.slots.swap_remove(physical_index_usize);

		if physical_index_u32 < self.len() {
			//swap_remove moved the last slot to the emptied slot.
			//need to update its random_access
			*self
				.random_access
				.get_mut(&self.slots[physical_index_usize].0)
				.unwrap() = physical_index_u32;
		}

		Some(removed_slot)
	}

	pub fn clear(&mut self) -> usize32 {
		let len = self.len();
		if len == 0 {
			return 0;
		}

		self.slots.clear();
		self.random_access.clear();
		self.next_id = 0;

		len
	}

	pub fn has(&self, id: usize32) -> bool {
		self.random_access.contains_key(&id)
	}

	pub fn get(&self, id: usize32) -> Option<&V> {
		self.random_access
			.get(&id)
			.map(|&physical_index| &self.slots.get(physical_index as usize).unwrap().1)
	}

	pub fn get_mut(&mut self, id: usize32) -> Option<&mut V> {
		self.random_access
			.get(&id)
			.map(|&physical_index| &mut self.slots.get_mut(physical_index as usize).unwrap().1)
	}

	pub fn len(&self) -> usize32 {
		self.slots.len() as usize32
	}

	pub fn iter(&self) -> impl ExactSizeIterator<Item = (usize32, &V)> {
		self.slots.iter().map(|slot| (slot.0, &slot.1))
	}

	pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = (usize32, &mut V)> {
		self.slots.iter_mut().map(|slot| (slot.0, &mut slot.1))
	}

	pub fn ids(&self) -> impl ExactSizeIterator<Item = usize32> {
		self.slots.iter().map(|slot| slot.0)
	}

	pub fn values(&self) -> impl ExactSizeIterator<Item = &V> {
		self.slots.iter().map(|slot| &slot.1)
	}

	pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut V> {
		self.slots.iter_mut().map(|slot| &mut slot.1)
	}
}

//---simulation_state---//

#[derive(Debug)]
pub struct SlotMap<V: TrackedState> {
	diff_path: Rc<Vec<usize32>>,
	field_id: usize32,

	#[cfg(feature = "server")]
	visibility: NetVisibility,

	data: RawSlotMap<V>,
}

//---constructors---//

impl<V: TrackedState> ConstructCollectionOrUtilityType for SlotMap<V> {
	fn construct(
		path: &Rc<Vec<usize32>>,
		field_id: usize32,

		#[cfg(feature = "server")] visibility: NetVisibility,
	) -> Self {
		Self {
			diff_path: path.clone(),
			field_id,

			#[cfg(feature = "server")]
			visibility,

			data: RawSlotMap::new(),
		}
	}
}

//---diff_ser---//

impl<V: TrackedState> Deref for SlotMap<V> {
	type Target = RawSlotMap<V>;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl<V: TrackedState> SlotMap<V> {
	pub fn add(&mut self, diff: &mut DiffSerializer<impl AnyTradeoff>) -> (usize32, &mut V) {
		self.add_with_client_owned(ClientKind::NA, diff)
	}

	pub(crate) fn add_with_client_owned(
		&mut self,
		client_kind: ClientKind,
		diff: &mut DiffSerializer<impl AnyTradeoff>,
	) -> (usize32, &mut V) {
		let op = DiffOperation::TrackSlotMapAdd;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self.diff_path) {
			self.data.next_id.ser_rollback(buffer);
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self.diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
		}

		self.data
			.add_with_id(|id| V::construct(&build_slot_path(id, &self.diff_path, self.field_id), client_kind))
	}

	//can't return the removed value! it would provide
	//a loophole for constructing new state objects.
	//the simulation state must own all instances.
	//unwrap to assert success
	#[must_use]
	pub fn remove(&mut self, id: usize32, diff: &mut DiffSerializer<impl AnyTradeoff>) -> Option<()> {
		let physical_index = *self.data.random_access.get(&id)?;
		let removed_slot = self.data.remove(id).unwrap();

		let op = DiffOperation::TrackSlotMapRemove;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self.diff_path) {
			removed_slot.ser_rollback_predict_remove(buffer);
			physical_index.ser_rollback(buffer);
			id.ser_rollback(buffer);
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self.diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
			id.ser_tx(buffer);
		}

		Some(())
	}

	//significantly more efficient in terms of bandwidth usage
	//compared to iter+remove
	pub fn clear(&mut self, diff: &mut DiffSerializer<impl AnyTradeoff>) -> usize32 {
		let len = self.data.len();
		if len == 0 {
			return 0;
		}

		let op = DiffOperation::TrackSlotMapClear;
		let diff = diff.to_impl();

		if let Some(buffer) = diff.ser_rollback_begin(&self.diff_path) {
			self.ser_rollback_predict_remove(buffer);
			self.field_id.ser_rollback(buffer);
			op.ser_rollback(buffer);
		}

		#[cfg(feature = "server")]
		for buffer in diff.ser_tx_begin(&self.diff_path, self.visibility) {
			op.ser_tx(buffer);
			self.field_id.ser_tx(buffer);
		}

		self.data.clear();
		len
	}

	pub fn get_mut(&mut self, id: usize32) -> Option<&mut V> {
		self.data.get_mut(id)
	}

	pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = (usize32, &mut V)> {
		self.data.iter_mut()
	}

	pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut V> {
		self.data.values_mut()
	}
}

//---diff_des---//

//wrapper to eliminate slotmap's generics and allow
//DiffDeserializeState trait to be dyn compatible
pub(crate) trait SlotMapDynCompat {
	fn get_des(&mut self, id: usize32) -> Option<&mut dyn DiffDeserializeState>;

	fn rollback_add(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;
	fn rollback_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;
	fn rollback_clear(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy>;

	#[cfg(feature = "client")]
	fn rx_add(&mut self, diff: &mut DiffSerializer<Impl>);
	#[cfg(feature = "client")]
	fn rx_remove(
		&mut self,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy>;
	#[cfg(feature = "client")]
	fn rx_clear(&mut self, diff: &mut DiffSerializer<Impl>);
}

impl<V: TrackedState> SlotMapDynCompat for SlotMap<V> {
	fn get_des(&mut self, id: usize32) -> Option<&mut dyn DiffDeserializeState> {
		self.get_mut(id).map(|v| v as &mut dyn DiffDeserializeState)
	}

	fn rollback_add(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		self.data.next_id = usize32::des_rollback(buffer)?;
		let id = self.data.slots.pop().unwrap().0;
		self.data.random_access.remove(&id).unwrap();
		Ok(())
	}

	fn rollback_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		let id = usize32::des_rollback(buffer)?;
		let physical_index = usize32::des_rollback(buffer)?;

		//since client deletion is unpredictable and will
		//never roll back, client_kind can be anything
		//here. all other constructors ignore the arg
		let mut value = V::construct(
			&build_slot_path(id, &self.diff_path, self.field_id),
			ClientKind::NA,
		);
		value.des_rollback_predict_remove(buffer)?;

		let reincarnated_slot = (id, value);
		if physical_index == self.data.len() {
			//insert as last slot
			self.data.slots.push(reincarnated_slot);
		} else {
			//insert in the middle + move the current resident
			//of this slot back to the end
			let end_slot = mem::replace(&mut self.data.slots[physical_index as usize], reincarnated_slot);
			*self.data.random_access.get_mut(&end_slot.0).unwrap() = self.data.len();
			self.data.slots.push(end_slot);
		}

		self.data.random_access.insert(id, physical_index);

		Ok(())
	}

	fn rollback_clear(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		self.des_rollback_predict_remove(buffer)
	}

	#[cfg(feature = "client")]
	fn rx_add(&mut self, diff: &mut DiffSerializer<Impl>) {
		self.add(diff);
	}

	#[cfg(feature = "client")]
	fn rx_remove(
		&mut self,
		buffer: &mut vec::IntoIter<u8>,
		diff: &mut DiffSerializer<Impl>,
	) -> Result<(), DeserializeOopsy> {
		let id = usize32::des_rx(buffer)?;
		self.remove(id, diff).ok_or(DeserializeOopsy::PathNotFound)
	}

	#[cfg(feature = "client")]
	fn rx_clear(&mut self, diff: &mut DiffSerializer<Impl>) {
		self.clear(diff);
	}
}

//---snapshot---//

impl<V: TrackedState> SnapshotState for SlotMap<V> {
	#[cfg(feature = "server")]
	fn ser_tx_new_client(&self, client_id: usize32, buffer: &mut Vec<u8>) {
		self.data.next_id.ser_tx(buffer);
		self.len().ser_tx(buffer);

		for slot in self.data.slots.iter() {
			slot.0.ser_tx(buffer);
			slot.1.ser_tx_new_client(client_id, buffer);
		}
	}

	#[cfg(feature = "client")]
	fn des_rx_new_client(
		&mut self,
		client_id: usize32,
		buffer: &mut impl Iterator<Item = u8>,
	) -> Result<(), DeserializeOopsy> {
		self.data.next_id = usize32::des_rx(buffer)?;
		let len = usize32::des_rx(buffer)?;

		for physical_index in 0..len {
			let id = usize32::des_rx(buffer)?;

			//nasty specialization hack: this is temporary,
			//with the thinking that the custom net visibility
			//scopes system will supersede owned/remote system
			let client_kind = if id == client_id && TypeId::of::<V>() == TypeId::of::<Client>() {
				ClientKind::Owned
			} else {
				ClientKind::Remote
			};

			let mut slot = V::construct(&build_slot_path(id, &self.diff_path, self.field_id), client_kind);
			slot.des_rx_new_client(client_id, buffer)?;

			self.data.slots.push((id, slot));
			self.data.random_access.insert(id, physical_index);
		}

		Ok(())
	}

	//this is expensive because it serializes every single
	//slot. if called, you've either manually called .clear(),
	//or a parent/outer collection called remove() and was
	//forced to serialize all inner collections. there is
	//potentially a large amount of data being rapidly rolled
	//back
	fn ser_rollback_predict_remove(&self, buffer: &mut Vec<u8>) {
		for slot in self.data.slots.iter().rev() {
			slot.1.ser_rollback_predict_remove(buffer);
			slot.0.ser_rollback(buffer);
		}

		self.data.len().ser_rollback(buffer);
		self.data.next_id.ser_rollback(buffer);
	}

	fn des_rollback_predict_remove(&mut self, buffer: &mut Vec<u8>) -> Result<(), DeserializeOopsy> {
		self.data.next_id = usize32::des_rollback(buffer)?;
		let slots_len = usize32::des_rollback(buffer)?;

		for physical_index in 0..slots_len {
			let id = usize32::des_rollback(buffer)?;
			let mut slot = V::construct(
				&build_slot_path(id, &self.diff_path, self.field_id),
				ClientKind::NA,
			);
			slot.des_rollback_predict_remove(buffer)?;

			self.data.slots.push((id, slot));
			self.data.random_access.insert(id, physical_index);
		}

		Ok(())
	}
}

//---untracked---//
impl<V: TrackedState> UntrackedState for SlotMap<V> {
	fn reset_untracked(&mut self) {
		for slot in self.values_mut() {
			slot.reset_untracked();
		}
	}
}

//---presentation_state---//

#[cfg(feature = "client")]
impl<V> PresentTick for SlotMap<V>
where
	V: TrackedState + PresentTick,
{
	type PresentationState = RawSlotMap<V::PresentationState>;
	fn clone_to_presentation(&self, tick: TickID) -> Self::PresentationState {
		RawSlotMap {
			slots: self
				.data
				.slots
				.iter()
				.map(|slot| (slot.0, slot.1.clone_to_presentation(tick)))
				.collect(),

			random_access: self.data.random_access.clone(),
			next_id: self.data.next_id,
		}
	}
}

//---interpolation---//

#[cfg(feature = "client")]
pub struct InterpolationSlotMap<V> {
	pub data: RawSlotMap<V>,
	pub slots_ptr: *const (usize32, V),
	pub slots_len: usize32,
	pub removed: Vec<usize32>,
	pub removed_ptr: *const usize32,
	pub removed_len: usize32,
	pub added: Vec<usize32>,
	pub added_ptr: *const usize32,
	pub added_len: usize32,
}

#[cfg(feature = "client")]
impl<V: InterpolateTicks> InterpolateTicks for RawSlotMap<V> {
	type InterpolationState = InterpolationSlotMap<V::InterpolationState>;
	fn interpolate_and_diff(
		prv: Option<&Self>,
		cur: &Self,
		amount: f32,
		received_new_tick: bool,
	) -> Self::InterpolationState {
		let Some(prv) = prv else {
			let slots: Vec<(u32, V::InterpolationState)> = cur
				.slots
				.iter()
				.map(|slot| {
					(
						slot.0,
						V::interpolate_and_diff(None, &slot.1, amount, received_new_tick),
					)
				})
				.collect();

			let slots_ptr = slots.as_ptr();
			let added: Vec<_> = cur.ids().collect();
			let added_ptr = added.as_ptr();

			return InterpolationSlotMap {
				data: RawSlotMap {
					slots,
					random_access: cur.random_access.clone(),
					next_id: cur.next_id,
				},

				slots_ptr,
				slots_len: cur.len(),
				removed: Vec::default(),
				removed_ptr: ptr::null(),
				removed_len: 0,
				added,
				added_ptr,
				added_len: cur.len(),
			};
		};

		let mut slots: Vec<MaybeUninit<(u32, V::InterpolationState)>> =
			(0..cur.slots.len()).map(|_a| MaybeUninit::uninit()).collect();

		let mut removed = Vec::new();
		let mut added = Vec::new();

		let max_len = prv.slots.len().max(cur.slots.len());
		for slot_idx in 0..max_len {
			//try to guess which slots in cur correspond to existing prv slots.
			//it is logically impossible to do this with complete accuracy
			//because the knowledge of how prv came to be cur is lost to time -
			//mispredicts causing id changes, rollbacks repeatedly triggering
			//add and remove events, etc. this algo simply searches for slots
			//between prv+cur that have matching id's and interpolates them if
			//found
			let prv_slot = prv.slots.get(slot_idx);
			let cur_slot = cur.slots.get(slot_idx);
			if let (Some(prv_slot), Some(cur_slot)) = (prv_slot, cur_slot)
				&& prv_slot.0 == cur_slot.0
			{
				//this branch should be what executes most often: data is still in
				//the same slot as the previous tick so can cleanly interpolate
				slots[slot_idx].write((
					cur_slot.0,
					V::interpolate_and_diff(Some(&prv_slot.1), &cur_slot.1, amount, received_new_tick),
				));
			} else {
				if let Some(cur_slot) = cur_slot {
					let prv_moved_slot = prv.get(cur_slot.0);
					slots[slot_idx].write((
						cur_slot.0,
						V::interpolate_and_diff(prv_moved_slot, &cur_slot.1, amount, received_new_tick),
					));

					if prv_moved_slot.is_none() && received_new_tick {
						added.push(cur_slot.0);
					}
				}

				if received_new_tick && let Some(prv_slot) = prv_slot {
					if !cur.has(prv_slot.0) {
						removed.push(prv_slot.0);
					}
				}
			}
		}

		//safety: for loop guarantees every slot was written to
		let slots: Vec<(u32, V::InterpolationState)> = unsafe { mem::transmute(slots) };
		let slots_ptr = slots.as_ptr();
		let removed_ptr = removed.as_ptr();
		let removed_len = removed.len() as usize32;
		let added_ptr = added.as_ptr();
		let added_len = added.len() as usize32;

		InterpolationSlotMap {
			data: RawSlotMap {
				slots,
				random_access: cur.random_access.clone(),
				next_id: cur.next_id,
			},

			slots_ptr,
			slots_len: cur.len(),
			removed,
			removed_ptr,
			removed_len,
			added,
			added_ptr,
			added_len,
		}
	}
}

//---misc---//

fn build_slot_path(id: usize32, diff_path: &Rc<Vec<usize32>>, field_id: usize32) -> Rc<Vec<usize32>> {
	let mut element_path = Vec::with_capacity(diff_path.len() + 2);
	element_path.extend(diff_path.iter());
	element_path.push(field_id);
	element_path.push(id);
	Rc::new(element_path)
}
