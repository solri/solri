use blockchain_core::Block as BlockT;
use alloc::vec::Vec;
#[cfg(feature = "parity-codec")]
use parity_codec::{Encode, Decode};

#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "parity-codec", derive(Encode, Decode))]
pub struct GenericBlock {
	pub id: Vec<u8>,
	pub parent_id: Option<Vec<u8>>,
	pub difficulty: u64,
	pub timestamp: u64,
	pub data: Vec<u8>,
}

impl BlockT for GenericBlock {
	type Identifier = Vec<u8>;

	fn parent_id(&self) -> Option<Vec<u8>> {
		self.parent_id.clone()
	}

	fn id(&self) -> Vec<u8> {
		self.id.clone()
	}
}
