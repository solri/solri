#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]
#![cfg_attr(not(feature = "std"), feature(core_intrinsics))]

extern crate alloc;

#[cfg(all(not(feature = "std"), target_arch = "wasm32"))]
mod wasm;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use alloc::{vec, vec::Vec};
use parity_codec::{Encode, Decode};
use blockchain_core::{Block as BlockT, BlockExecutor, SimpleBuilderExecutor, NullExternalities};
use sha3::Sha3_256;
use primitive_types::H256;
use bm_le::{FromTree, IntoTree, tree_root};
use metadata::GenericBlock;

#[derive(Debug)]
pub enum Error {
	InvalidBlock,
	DifficultyTooLow,
}

#[cfg(feature = "std")]
impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}

#[cfg(feature = "std")]
impl std::error::Error for Error { }

#[derive(Debug)]
pub struct Metadata {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_id: Vec<u8>,
	pub id: Vec<u8>,
}

const DIFFICULTY: usize = 0;

fn is_all_zero(arr: &[u8]) -> bool {
	arr.len() == 0 ||
		arr.iter().all(|i| *i == 0)
}

#[derive(Clone, Debug, Encode, Decode, FromTree, IntoTree)]
pub struct Header {
	pub parent: Option<H256>,
	pub timestamp: u64,
	pub state: u128,
	pub extrinsics: H256,
	pub nonce: u64,
}

impl Header {
	pub fn id(&self) -> H256 {
		tree_root::<Sha3_256, _>(self)
	}
}

impl From<Block> for Header {
	fn from(block: Block) -> Header {
		Header {
			parent: block.parent.map(|p| p.id()),
			timestamp: block.timestamp,
			state: block.state,
			extrinsics: tree_root::<Sha3_256, _>(&block.extrinsics),
			nonce: block.nonce,
		}
	}
}

#[derive(Clone, Debug)]
pub struct UnsealedBlock {
	pub parent: Option<Header>,
	pub timestamp: u64,
	pub state: u128,
	pub extrinsics: Vec<Extrinsic>,
}

impl UnsealedBlock {
	pub fn seal(self) -> Block {
		let mut block = Block {
			parent: self.parent,
			timestamp: self.timestamp,
			state: self.state,
			extrinsics: self.extrinsics,
			nonce: 0,
		};

		while !is_all_zero(&block.id()[0..DIFFICULTY]) {
			block.nonce += 1;
		}

		block
	}
}

#[derive(Clone, Debug, Encode, Decode, FromTree, IntoTree)]
pub struct Block {
	pub parent: Option<Header>,
	pub timestamp: u64,
	pub state: u128,
	pub extrinsics: Vec<Extrinsic>,
	pub nonce: u64,
}

impl Block {
	pub fn genesis() -> Self {
		Block {
			parent: None,
			timestamp: 0,
			state: 0,
			extrinsics: Vec::new(),
			nonce: 0,
		}
	}
}

impl BlockT for Block {
	type Identifier = H256;

	fn parent_id(&self) -> Option<H256> {
		self.parent.as_ref().map(|p| p.id())
	}

	fn id(&self) -> H256 {
		Header::from(self.clone()).id()
	}
}

impl Into<GenericBlock> for Block {
	fn into(self) -> GenericBlock {
		GenericBlock {
			id: self.id()[..].to_vec(),
			parent_id: self.parent_id().map(|p| p[..].to_vec()),
			difficulty: 1,
			timestamp: self.timestamp,
			data: self.encode(),
		}
	}
}

#[derive(Clone, Debug, FromTree, IntoTree, Encode, Decode)]
pub enum Extrinsic {
	Add(u128),
}

#[derive(Clone)]
pub struct Executor;

impl BlockExecutor for Executor {
	type Error = Error;
	type Block = Block;
	type Externalities = dyn NullExternalities + 'static;

	fn execute_block(
		&self,
		block: &Block,
		_state: &mut Self::Externalities,
	) -> Result<(), Error> {
		if !is_all_zero(&block.id()[0..DIFFICULTY]) {
			return Err(Error::DifficultyTooLow);
		}

		let mut counter = block.parent.as_ref().map(|p| p.state).unwrap_or(0);
		for extrinsic in &block.extrinsics {
			match extrinsic {
				Extrinsic::Add(add) => counter += add,
			}
		}

		if counter != block.state {
			return Err(Error::InvalidBlock)
		}

		Ok(())
	}
}

impl SimpleBuilderExecutor for Executor {
	type BuildBlock = UnsealedBlock;
	type Extrinsic = Extrinsic;
	type Inherent = u64;

	fn initialize_block(
		&self,
		parent_block: &Self::Block,
		_state: &mut Self::Externalities,
		inherent: u64,
	) -> Result<Self::BuildBlock, Self::Error> {
		let parent_state = parent_block.state;

		Ok(UnsealedBlock {
			state: parent_state,
			timestamp: inherent,
			parent: Some(parent_block.clone().into()),
			extrinsics: Vec::new(),
		})
	}

	fn apply_extrinsic(
		&self,
		block: &mut Self::BuildBlock,
		extrinsic: Self::Extrinsic,
		_state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
		match extrinsic {
			Extrinsic::Add(add) => block.state += add,
		}
		block.extrinsics.push(extrinsic);

		Ok(())
	}

	fn finalize_block(
		&self,
		_block: &mut Self::BuildBlock,
		_state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
		Ok(())
	}
}

pub fn execute(block: &[u8], _code: &mut Vec<u8>) -> Result<Metadata, Error> {
	let block = Block::decode(&mut &block[..]).ok_or(Error::InvalidBlock)?;
	let executor = Executor;

	executor.execute_block(&block, &mut ())?;

	Ok(Metadata {
		timestamp: block.timestamp,
		difficulty: 1,
		parent_id: match block.parent.as_ref().map(|p| p.id()) {
			Some(id) => id[..].to_vec(),
			None => vec![],
		},
		id: block.id()[..].to_vec(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn header_id_equal_id() {
		let block = Block::genesis();
		assert_eq!(tree_root::<Sha3_256, _>(&block),
				   tree_root::<Sha3_256, _>(&Header::from(block.clone())));
	}
}
