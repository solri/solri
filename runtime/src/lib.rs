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
use sha3::{Digest, Sha3_256};
use primitive_types::H256;

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
	pub parent_hash: Vec<u8>,
	pub hash: Vec<u8>,
}

const DIFFICULTY: usize = 1;

fn is_all_zero(arr: &[u8]) -> bool {
	arr.iter().all(|i| *i == 0)
}

#[derive(Clone, Debug)]
pub struct UnsealedBlock {
	pub parent_hash: Option<H256>,
	pub timestamp: u64,
	pub previous_counter: u128,
	pub extrinsics: Vec<Extrinsic>,
}

impl UnsealedBlock {
	pub fn seal(self) -> Block {
		let mut block = Block {
			parent_hash: self.parent_hash,
			extrinsics: self.extrinsics,
			timestamp: self.timestamp,
			previous_counter: self.previous_counter,
			nonce: 0,
		};

		while !is_all_zero(&block.id()[0..DIFFICULTY]) {
			block.nonce += 1;
		}

		block
	}
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Block {
	pub parent_hash: Option<H256>,
	pub timestamp: u64,
	pub previous_counter: u128,
	pub extrinsics: Vec<Extrinsic>,
	pub nonce: u64,
}

impl Block {
	pub fn genesis() -> Self {
		Block {
			parent_hash: None,
			timestamp: 0,
			previous_counter: 0,
			extrinsics: Vec::new(),
			nonce: 0,
		}
	}
}

impl BlockT for Block {
	type Identifier = H256;

	fn parent_id(&self) -> Option<H256> {
		self.parent_hash
	}

	fn id(&self) -> H256 {
		H256::from_slice(Sha3_256::digest(&self.encode()).as_slice())
	}
}

#[derive(Clone, Debug, Encode, Decode)]
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

		let mut counter = block.previous_counter;
		for extrinsic in &block.extrinsics {
			match extrinsic {
				Extrinsic::Add(add) => counter += add,
			}
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
		block: &Self::Block,
		_state: &mut Self::Externalities,
		inherent: u64,
	) -> Result<Self::BuildBlock, Self::Error> {
		let mut counter = block.previous_counter;
		for extrinsic in &block.extrinsics {
			match extrinsic {
				Extrinsic::Add(add) => counter += add,
			}
		}

		Ok(UnsealedBlock {
			previous_counter: counter,
			timestamp: inherent,
			parent_hash: Some(block.id()),
			extrinsics: Vec::new(),
		})
	}

	fn apply_extrinsic(
		&self,
		block: &mut Self::BuildBlock,
		extrinsic: Self::Extrinsic,
		_state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
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
		parent_hash: match block.parent_hash {
			Some(hash) => hash[..].to_vec(),
			None => vec![],
		},
		hash: block.id()[..].to_vec(),
	})
}
