#![no_std]

use core::mem;

pub struct RawMetadata {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_hash_ptr: u32,
	pub parent_hash_len: u32,
	pub hash_ptr: u32,
	pub hash_len: u32,
	pub code_ptr: u32,
	pub code_len: u32,
}

impl RawMetadata {
	pub fn bytes_len() -> usize {
		mem::size_of::<u64>() + // timestamp
			mem::size_of::<u64>() + // difficulty
			mem::size_of::<u32>() + // parent_hash_ptr
			mem::size_of::<u32>() + // parent_hash_len
			mem::size_of::<u32>() + // hash_ptr
			mem::size_of::<u32>() + // hash_len
			mem::size_of::<u32>() + // code_ptr
			mem::size_of::<u32>() // code_len
	}

	pub fn decode(bytes: &[u8]) -> Option<Self> {
		fn decode_u64(bytes: &[u8]) -> Option<u64> {
			let mut arr = 0u64.to_le_bytes();
			if arr.len() != bytes.len() {
				return None
			}
			arr.copy_from_slice(bytes);
			Some(u64::from_le_bytes(arr))
		}

		fn decode_u32(bytes: &[u8]) -> Option<u32> {
			let mut arr = 0u32.to_le_bytes();
			if arr.len() != bytes.len() {
				return None
			}
			arr.copy_from_slice(bytes);
			Some(u32::from_le_bytes(arr))
		}

		Some(RawMetadata {
			timestamp: decode_u64(&bytes[0..8])?,
			difficulty: decode_u64(&bytes[8..16])?,
			parent_hash_ptr: decode_u32(&bytes[16..20])?,
			parent_hash_len: decode_u32(&bytes[20..24])?,
			hash_ptr: decode_u32(&bytes[24..28])?,
			hash_len: decode_u32(&bytes[28..32])?,
			code_ptr: decode_u32(&bytes[32..36])?,
			code_len: decode_u32(&bytes[36..40])?,
		})
	}
}
