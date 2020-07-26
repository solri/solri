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
use blockchain_core::{Block as BlockT, BlockExecutor, AsExternalities, ExtrinsicBuilder};
use sha3::Sha3_256;
use primitive_types::H256;
use bm::{
	CompactValue, ProvingState, Proofs, ReadBackend, WriteBackend, DynBackend,
	InMemoryBackend, ProvingBackend, Tree
};
use bm_le::{FromTree, IntoTree, Value, tree_root};
use metadata::GenericBlock;

pub type Construct = bm_le::DigestConstruct<Sha3_256>;

pub trait TrieExternalities {
	fn db(&self) -> &dyn ReadBackend<Construct=Construct, Error=()>;
	fn db_mut(&mut self) -> &mut dyn WriteBackend<Construct=Construct, Error=()>;
}

#[derive(Default)]
pub struct InMemoryTrie(DynBackend<InMemoryBackend<Construct>>);

impl TrieExternalities for InMemoryTrie {
	fn db(&self) -> &dyn ReadBackend<Construct=Construct, Error=()> {
		&self.0
	}

	fn db_mut(&mut self) -> &mut dyn WriteBackend<Construct=Construct, Error=()> {
		&mut self.0
	}
}
impl AsExternalities<dyn TrieExternalities> for InMemoryTrie {
	fn as_externalities(&mut self) -> &mut (dyn TrieExternalities + 'static) { self }
}

#[derive(Debug)]
pub enum Error {
	InvalidBlock,
	DifficultyTooLow,
	Backend
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
	pub state: H256,
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
			state: tree_root::<Sha3_256, _>(&block.state),
			extrinsics: tree_root::<Sha3_256, _>(&block.extrinsics),
			nonce: block.nonce,
		}
	}
}

#[derive(Clone, Debug)]
pub struct UnsealedBlock {
	pub parent: Option<Header>,
	pub timestamp: u64,
	pub parent_state: (Value, ProvingState<Value>),
	pub state: Value,
	pub extrinsics: Vec<Extrinsic>,
}

impl UnsealedBlock {
	pub fn seal(self) -> Block {
		let mut block = Block {
			parent: self.parent,
			timestamp: self.timestamp,
			parent_state: Proofs::from(self.parent_state.1).into_compact(self.parent_state.0),
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

#[derive(Clone, Debug, Encode, Decode, IntoTree)]
pub struct Block {
	pub parent: Option<Header>,
	pub timestamp: u64,
	pub parent_state: CompactValue<Value>,
	pub state: Value,
	pub extrinsics: Vec<Extrinsic>,
	pub nonce: u64,
}

impl Block {
	pub fn genesis() -> Self {
		Block {
			parent: None,
			timestamp: 0,
			parent_state: CompactValue::Single(Default::default()),
			state: Default::default(),
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
	Add(u64),
}

#[derive(Default, Clone)]
pub struct Executor;

impl BlockExecutor for Executor {
	type Error = Error;
	type Block = Block;
	type Externalities = dyn TrieExternalities + 'static;

	fn execute_block(
		&self,
		block: &Block,
		state: &mut Self::Externalities,
	) -> Result<(), Error> {
		if !is_all_zero(&block.id()[0..DIFFICULTY]) {
			return Err(Error::DifficultyTooLow);
		}

		let parent_state_root = Value(tree_root::<Sha3_256, _>(&block.parent_state));
		let state_root = block.state.clone();

		let mut trie = if parent_state_root == Default::default() {
			bm::List::create(state.db_mut(), None).map_err(|_| Error::Backend)?
		} else {
			bm::List::reconstruct(parent_state_root, state.db_mut(), None)
				.map_err(|_| Error::Backend)?
		};

		for extrinsic in &block.extrinsics {
			match extrinsic {
				Extrinsic::Add(add) => {
					trie.push(state.db_mut(), Value(H256::from_low_u64_le(*add)))
						.map_err(|_| Error::Backend)?;
				},
			}
		}

		if trie.root() != state_root {
			return Err(Error::InvalidBlock)
		}

		Ok(())
	}
}

impl ExtrinsicBuilder for Executor {
	type BuildBlock = UnsealedBlock;
	type Extrinsic = Extrinsic;
	type Inherent = u64;

	fn initialize_block(
		&self,
		parent_block: &Self::Block,
		state: &mut Self::Externalities,
		inherent: u64,
	) -> Result<Self::BuildBlock, Self::Error> {
		let parent_state_root = parent_block.state.clone();

		let mut proving = ProvingBackend::new(state.db_mut());
		let trie = if parent_state_root == Default::default() {
			bm::List::create(&mut proving, None).map_err(|_| Error::Backend)?
		} else {
			bm::List::reconstruct(parent_state_root.clone(), &mut proving, None)
				.map_err(|_| Error::Backend)?
		};
		let proving_state = proving.into_state();

		Ok(UnsealedBlock {
			state: trie.root(),
			parent_state: (parent_state_root, proving_state),
			timestamp: inherent,
			parent: Some(parent_block.clone().into()),
			extrinsics: Vec::new(),
		})
	}

	fn apply_extrinsic(
		&self,
		block: &mut Self::BuildBlock,
		extrinsic: Self::Extrinsic,
		state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
		let mut proving = ProvingBackend::from_state(block.parent_state.1.clone(), state.db_mut());
		let mut trie = bm::OwnedList::reconstruct(block.state.clone(), &mut proving, None)
			.map_err(|_| Error::Backend)?;

		match extrinsic {
			Extrinsic::Add(add) => {
				trie.push(&mut proving, Value(H256::from_low_u64_le(add)))
					.map_err(|_| Error::Backend)?;
			},
		}
		block.extrinsics.push(extrinsic);
		block.parent_state.1 = proving.into_state();
		block.state = trie.root();

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
	let (proofs, _) = Proofs::from_compact::<Construct>(block.parent_state.clone());
	let executor = Executor;
	let mut trie = InMemoryTrie::default();
	trie.0.populate(proofs.into());

	executor.execute_block(&block, &mut trie)?;

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
	#[ignore]
	fn header_id_equal_id() {
		let block = Block::genesis();
		assert_eq!(
			tree_root::<Sha3_256, _>(&block),
			tree_root::<Sha3_256, _>(&Header::from(block.clone()))
		);
	}
}
