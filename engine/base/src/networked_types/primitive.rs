use crate::context::Impl;
use crate::diff_ser::DiffSerializer;
use crate::tick::TickType;
use crate::{DeserializeOopsy, DiffOperation};
use glam::{DQuat, DVec2, DVec3, Quat, Vec2, Vec3, Vec3A};
use std::mem::MaybeUninit;
use std::rc::Rc;

#[cfg(feature = "server")]
use crate::NetVisibility;

//the server frequently needs to send usize data to
//the client. unfortunately the server is usually
//64 bit while the client is 32 bit. there really
//isn't a graceful way of handling this because
//clients simply can't address > 4gb memory. so,
//i've chosen to just crash the server here in the
//event that it tries to tell clients to do
//something impossible
#[allow(non_camel_case_types)]
pub type usize32 = u32;
#[allow(non_camel_case_types)]
pub type isize32 = i32;

#[cfg(target_pointer_width = "64")]
const PTR_ERR: &str = "Server must abort because it is using more memory than a client can reference. The server is 64-bit, but the client is 32-bit (wasm32)";

pub fn usize_to_32(v: usize) -> usize32 {
	#[cfg(target_pointer_width = "32")]
	return v as usize32;

	#[cfg(target_pointer_width = "64")]
	return usize32::try_from(v).expect(PTR_ERR);
}

//note field_id is technically part of the path but
//is passed as a separate parameter for optimization
//purposes (avoid having a vec for every single
//field, avoid changing path when writing to
//multiple fields on the same struct).
//
//rollback_prv_value: represents data that will be
//rolled back. the state's previous value will be
//written
//
//tx_new_value: represents data that will be sent
//over the wire. the state's new value will be
//written. the visibility and path arguments will
//determine who it's sent to
pub(crate) fn ser_sim_primitive<T: PrimitiveSerDes>(
	diff: &mut DiffSerializer<Impl>,
	path: &Rc<Vec<usize32>>,
	field_id: usize32,
	rollback_prv_value: T,

	#[cfg(feature = "server")] visibility: NetVisibility,
	#[cfg(feature = "server")] tx_new_value: T,
) {
	let op = DiffOperation::TrackPrimitive;

	if let Some(buffer) = diff.ser_rollback_begin(path) {
		rollback_prv_value.ser_rollback(buffer);
		field_id.ser_rollback(buffer);
		op.ser_rollback(buffer);
	}

	#[cfg(feature = "server")]
	for buffer in diff.ser_tx_begin(path, visibility) {
		op.ser_tx(buffer);
		field_id.ser_tx(buffer);
		tx_new_value.ser_tx(buffer);
	}
}

//slimmed down version of ser_sim_primitive specifically for
//clients writing to their input states, which only have
///primitive types, can only do DiffOperation::TrackPrimitive,
//and never roll back
#[cfg(feature = "client")]
pub(crate) fn ser_input_primitive<T: PrimitiveSerDes>(
	diff: &mut DiffSerializer<Impl>,
	field_id: usize32,
	tx_new_value: T,
) {
	let buffer = diff.ser_tx_begin();
	field_id.ser_tx(buffer);
	tx_new_value.ser_tx(buffer);
}

//implementors list taken from Validator.ts/rustPrimitiveSchema
//rollback - read with pop_back. rollback data stays local so compression is not as necessary
//tx = read with pop_front. some primitives have special compression strategies to reduce bandwidth
pub(crate) trait PrimitiveSerDes: Copy + 'static {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		//no compression. copy bytes directly into the buffer
		buffer.extend(ser_raw_bytes(&self));
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		let old_len = buffer.len();
		let read = size_of::<Self>();

		if old_len < read {
			return Err(DeserializeOopsy::BufferUnderflow);
		}

		let new_len = old_len - read;
		let data = unsafe { (buffer.as_ptr().add(new_len) as *const Self).read_unaligned() };
		buffer.truncate(new_len);

		Ok(data)
	}

	fn ser_tx(self, buffer: &mut Vec<u8>) {
		self.ser_rollback(buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		let mut data = MaybeUninit::<Self>::uninit();
		let bytes =
			unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, size_of::<Self>()) };

		for byte in bytes.iter_mut() {
			*byte = buffer.next().ok_or(DeserializeOopsy::BufferUnderflow)?;
		}

		Ok(unsafe { data.assume_init() })
	}
}

//safety: all RustPrimitive types are pod
fn ser_raw_bytes<T: PrimitiveSerDes>(data: &T) -> &[u8] {
	unsafe { std::slice::from_raw_parts(data as *const T as *const u8, size_of_val(data)) }
}

//nasty ancient rust bug necessitates lots
//of duplicate impls
//https://github.com/rust-lang/rust/issues/20400

impl PrimitiveSerDes for bool {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		buffer.push(self.into());
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		des_bool(buffer.pop().ok_or(DeserializeOopsy::BufferUnderflow)?)
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_bool(buffer.next().ok_or(DeserializeOopsy::BufferUnderflow)?)
	}
}

fn des_bool(data: u8) -> Result<bool, DeserializeOopsy> {
	match data {
		0 => Ok(false),
		1 => Ok(true),
		_ => Err(DeserializeOopsy::CorruptBool),
	}
}

impl PrimitiveSerDes for u8 {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		buffer.push(self);
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		buffer.pop().ok_or(DeserializeOopsy::BufferUnderflow)
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		buffer.next().ok_or(DeserializeOopsy::BufferUnderflow)
	}
}

impl PrimitiveSerDes for i8 {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		buffer.push(self as u8);
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		Ok(buffer.pop().ok_or(DeserializeOopsy::BufferUnderflow)? as i8)
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		Ok(buffer.next().ok_or(DeserializeOopsy::BufferUnderflow)? as i8)
	}
}

impl PrimitiveSerDes for DiffOperation {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		buffer.push(self.into());
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		buffer
			.pop()
			.ok_or(DeserializeOopsy::BufferUnderflow)?
			.try_into()
			.map_err(|_| DeserializeOopsy::CorruptDiffOperation)
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		buffer
			.next()
			.ok_or(DeserializeOopsy::BufferUnderflow)?
			.try_into()
			.map_err(|_| DeserializeOopsy::CorruptDiffOperation)
	}
}

impl PrimitiveSerDes for TickType {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		buffer.push(self.into());
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		buffer
			.pop()
			.ok_or(DeserializeOopsy::BufferUnderflow)?
			.try_into()
			.map_err(|_| DeserializeOopsy::CorruptTickType)
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		buffer
			.next()
			.ok_or(DeserializeOopsy::BufferUnderflow)?
			.try_into()
			.map_err(|_| DeserializeOopsy::CorruptTickType)
	}
}

impl PrimitiveSerDes for u16 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_u(self.into(), buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_u(buffer)?
			.try_into()
			.map_err(|_| DeserializeOopsy::ObeseVarInt)
	}
}

impl PrimitiveSerDes for i16 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_i(self.into(), buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_i(buffer)?
			.try_into()
			.map_err(|_| DeserializeOopsy::ObeseVarInt)
	}
}

impl PrimitiveSerDes for u32 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_u(self.into(), buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_u(buffer)?
			.try_into()
			.map_err(|_| DeserializeOopsy::ObeseVarInt)
	}
}

impl PrimitiveSerDes for i32 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_i(self.into(), buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_i(buffer)?
			.try_into()
			.map_err(|_| DeserializeOopsy::ObeseVarInt)
	}
}

impl PrimitiveSerDes for u64 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_u(self, buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_u(buffer)
	}
}

impl PrimitiveSerDes for i64 {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_i(self, buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		des_varint_i(buffer)
	}
}

const MSB: u8 = 0b10000000;
const DROP_MSB: u8 = 0b01111111;

//unsigned varint serialization/encoding/compression
//https://docs.rs/integer-encoding/4.0.2/src/integer_encoding/varint.rs.html#156
fn ser_varint_u(mut data: u64, buffer: &mut Vec<u8>) {
	while data >= 0x80 {
		buffer.push(MSB | (data as u8));
		data >>= 7;
	}

	buffer.push(data as u8);
}

//unsigned varint deserialization/decoding/decompression
//https://docs.rs/integer-encoding/4.0.2/src/integer_encoding/varint.rs.html#132
fn des_varint_u(buffer: &mut impl Iterator<Item = u8>) -> Result<u64, DeserializeOopsy> {
	let mut result: u64 = 0;
	let mut shift = 0;

	let mut success = false;
	while let Some(b) = buffer.next() {
		let msb_dropped = b & DROP_MSB;
		result |= (msb_dropped as u64) << shift;
		shift += 7;

		if b & MSB == 0 || shift > (9 * 7) {
			success = b & MSB == 0;
			break;
		}
	}

	if success {
		Ok(result)
	} else {
		Err(DeserializeOopsy::CorruptVarInt)
	}
}

//signed varint+zigzag/encoding/compression
//https://docs.rs/integer-encoding/4.0.2/src/integer_encoding/varint.rs.html#57
fn ser_varint_i(data: i64, buffer: &mut Vec<u8>) {
	let data: i64 = data.into();
	ser_varint_u(((data << 1) ^ (data >> 63)) as u64, buffer);
}

//signed varint+zigzag/encoding/compression
//https://docs.rs/integer-encoding/4.0.2/src/integer_encoding/varint.rs.html#65
fn des_varint_i(buffer: &mut impl Iterator<Item = u8>) -> Result<i64, DeserializeOopsy> {
	let data = des_varint_u(buffer)?;
	Ok(((data >> 1) ^ (-((data & 1) as i64)) as u64) as i64)
}

impl PrimitiveSerDes for f32 {}
impl PrimitiveSerDes for f64 {}

impl PrimitiveSerDes for char {
	fn ser_tx(self, buffer: &mut Vec<u8>) {
		ser_varint_u(self.into(), buffer);
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		let as_u64 = des_varint_u(buffer)?;
		let as_u32: u32 = as_u64.try_into().map_err(|_| DeserializeOopsy::ObeseVarInt)?;
		let as_char: char = as_u32.try_into().map_err(|_| DeserializeOopsy::CorruptChar)?;
		Ok(as_char)
	}
}

impl PrimitiveSerDes for Vec2 {}
impl PrimitiveSerDes for DVec2 {}
impl PrimitiveSerDes for Vec3 {}

impl PrimitiveSerDes for Vec3A {
	fn ser_rollback(self, buffer: &mut Vec<u8>) {
		Vec3::from(self).ser_rollback(buffer);
	}

	fn des_rollback(buffer: &mut Vec<u8>) -> Result<Self, DeserializeOopsy> {
		Vec3::des_rollback(buffer).map(|v| v.into())
	}

	fn des_rx(buffer: &mut impl Iterator<Item = u8>) -> Result<Self, DeserializeOopsy> {
		Vec3::des_rx(buffer).map(|v| v.into())
	}
}

impl PrimitiveSerDes for DVec3 {}
impl PrimitiveSerDes for Quat {}
impl PrimitiveSerDes for DQuat {}

//it is the caller's responsibility to ser/des the slice length
pub(crate) trait SliceSerDes<T: PrimitiveSerDes>: AsRef<[T]> {
	fn ser_rollback(&self, buffer: &mut Vec<u8>) {
		for &data in self.as_ref().iter().rev() {
			data.ser_rollback(buffer);
		}
	}

	fn des_rollback(len: usize32, buffer: &mut Vec<u8>) -> Result<Vec<T>, DeserializeOopsy> {
		let mut out = Vec::with_capacity(len as usize);
		for _ in 0..len {
			out.push(T::des_rollback(buffer)?);
		}

		Ok(out)
	}

	#[allow(dead_code)]
	fn ser_tx(&self, buffer: &mut Vec<u8>) {
		for &data in self.as_ref() {
			data.ser_tx(buffer);
		}
	}

	#[allow(dead_code)]
	fn des_rx(len: usize32, buffer: &mut impl Iterator<Item = u8>) -> Result<Vec<T>, DeserializeOopsy> {
		let mut out = Vec::with_capacity(len as usize);
		for _ in 0..len {
			out.push(T::des_rx(buffer)?);
		}

		Ok(out)
	}
}

//bit packing
impl SliceSerDes<bool> for [bool] {
	fn ser_tx(&self, buffer: &mut Vec<u8>) {
		for chunk in self.chunks(8) {
			buffer.push(
				chunk
					.iter()
					.enumerate()
					.fold(0, |acc, (i, &bit)| acc | ((bit as u8) << (7 - i))),
			);
		}
	}

	fn des_rx(
		bool_len: usize32,
		buffer: &mut impl Iterator<Item = u8>,
	) -> Result<Vec<bool>, DeserializeOopsy> {
		let bool_len = bool_len as usize;
		let byte_len = bool_len.div_ceil(8);
		let mut out = Vec::with_capacity(bool_len);

		for _ in 0..byte_len {
			let byte = buffer.next().ok_or(DeserializeOopsy::BufferUnderflow)?;
			for i in 0..8 {
				if out.len() == bool_len
				//only extract bits up to the original length
				{
					break;
				}

				let bit = ((byte >> (7 - i)) & 1) != 0;
				out.push(bit);
			}
		}

		Ok(out)
	}
}

impl SliceSerDes<u8> for [u8] {}
impl SliceSerDes<i8> for [i8] {}
impl SliceSerDes<u16> for [u16] {}
impl SliceSerDes<i16> for [i16] {}
impl SliceSerDes<u32> for [u32] {}
impl SliceSerDes<i32> for [i32] {}
impl SliceSerDes<u64> for [u64] {}
impl SliceSerDes<i64> for [i64] {}
impl SliceSerDes<f32> for [f32] {}
impl SliceSerDes<f64> for [f64] {}
impl SliceSerDes<char> for [char] {}
impl SliceSerDes<Vec2> for [Vec2] {}
impl SliceSerDes<DVec2> for [DVec2] {}
impl SliceSerDes<Vec3> for [Vec3] {}
impl SliceSerDes<Vec3A> for [Vec3A] {}
impl SliceSerDes<DVec3> for [DVec3] {}
impl SliceSerDes<Quat> for [Quat] {}
impl SliceSerDes<DQuat> for [DQuat] {}
