use core::mem;
use alloc::vec::Vec;

pub struct RawArray {
	pub ptr: u32,
	pub len: u32,
}

impl RawArray {
	pub fn bytes_len() -> usize {
		mem::size_of::<u32>() + mem::size_of::<u32>()
	}

	pub fn decode(bytes: &[u8]) -> Option<Self> {
		fn decode_u32(bytes: &[u8]) -> Option<u32> {
			let mut arr = 0u32.to_le_bytes();
			if arr.len() != bytes.len() {
				return None
			}
			arr.copy_from_slice(bytes);
			Some(u32::from_le_bytes(arr))
		}

		Some(RawArray {
			ptr: decode_u32(&bytes[0..4])?,
			len: decode_u32(&bytes[4..8])?,
		})
	}

	pub fn encode(&self) -> Vec<u8> {
		fn encode_u32(value: u32) -> Vec<u8> {
			value.to_le_bytes().to_vec()
		}

		let mut ret = Vec::new();
		ret.append(&mut encode_u32(self.ptr));
		ret.append(&mut encode_u32(self.len));
		ret
	}
}

pub struct RawMetadata {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_id: RawArray,
	pub id: RawArray,
	pub code: RawArray,
}

impl RawMetadata {
	pub fn bytes_len() -> usize {
		mem::size_of::<u64>() + // timestamp
			mem::size_of::<u64>() + // difficulty
			RawArray::bytes_len() + // parent_id
			RawArray::bytes_len() + // id
			RawArray::bytes_len() // code
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

		Some(RawMetadata {
			timestamp: decode_u64(&bytes[0..8])?,
			difficulty: decode_u64(&bytes[8..16])?,
			parent_id: RawArray::decode(&bytes[16..24])?,
			id: RawArray::decode(&bytes[24..32])?,
			code: RawArray::decode(&bytes[32..40])?,
		})
	}

	pub fn encode(&self) -> Vec<u8> {
		fn encode_u64(value: u64) -> Vec<u8> {
			value.to_le_bytes().to_vec()
		}

		let mut ret = Vec::new();
		ret.append(&mut encode_u64(self.timestamp));
		ret.append(&mut encode_u64(self.difficulty));
		ret.append(&mut self.parent_id.encode());
		ret.append(&mut self.id.encode());
		ret.append(&mut self.code.encode());
		ret
	}
}
