#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

//! SimpleSerialize (ssz) compliant binary merkle tree supporting both
//! merkleization and de-merkleization.

extern crate alloc;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use typenum::U32;
use generic_array::GenericArray;
use primitive_types::H256;
use digest::Digest;

pub use bm::{
	Backend, ReadBackend, WriteBackend, InheritedDigestConstruct,
	UnitDigestConstruct, Construct, InheritedEmpty, Error, Vector,
	DanglingVector, List, Leak, NoopBackend, InMemoryBackend
};

mod basic;
mod elemental_fixed;
mod elemental_variable;
mod fixed;
mod variable;
pub mod utils;

pub use elemental_fixed::{
	ElementalFixedVec, ElementalFixedVecRef,
	IntoCompactVectorTree, FromCompactVectorTree,
	IntoCompositeVectorTree, FromCompositeVectorTree
};
pub use elemental_variable::{
	ElementalVariableVec, ElementalVariableVecRef,
	IntoCompactListTree, FromCompactListTree,
	IntoCompositeListTree, FromCompositeListTree
};
pub use variable::MaxVec;
#[cfg(feature = "derive")]
pub use bm_le_derive::{FromTree, IntoTree};

/// Digest construct for bm-le.
pub type DigestConstruct<D> = bm::InheritedDigestConstruct<D, Value>;

/// End value for 256-bit ssz binary merkle tree.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-codec", derive(parity_codec::Encode, parity_codec::Decode))]
pub struct Value(pub H256);

impl Default for Value {
	fn default() -> Self {
		Self(H256::default())
	}
}

impl AsRef<[u8]> for Value {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl AsMut<[u8]> for Value {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl From<usize> for Value {
	fn from(value: usize) -> Self {
		let mut ret = [0u8; 32];
		let bytes = (value as u64).to_le_bytes();
		(&mut ret[0..8]).copy_from_slice(&bytes);
		Value(H256::from(ret))
	}
}

impl Into<usize> for Value {
	fn into(self) -> usize {
		let mut raw = [0u8; 8];
		(&mut raw).copy_from_slice(&self.0[0..8]);
		u64::from_le_bytes(raw) as usize
	}
}

impl From<GenericArray<u8, typenum::U32>> for Value {
	fn from(array: GenericArray<u8, typenum::U32>) -> Self {
		Self(H256::from_slice(array.as_slice()))
	}
}

/// Intermediate type for 256-bit ssz binary merkle tree.
pub type Intermediate = H256;

/// Special type for le-compatible construct.
pub trait CompatibleConstruct: Construct<Value=Value> { }

impl<C: Construct<Value=Value>> CompatibleConstruct for C { }

/// Traits for type converting into a tree structure.
pub trait IntoTree {
	/// Convert this type into merkle tree, writing nodes into the
	/// given database.
	fn into_tree<DB: WriteBackend>(
		&self,
		db: &mut DB
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for type converting from a tree structure.
pub trait FromTree: Sized {
	/// Convert this type from merkle tree, reading nodes from the
	/// given database.
	fn from_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Indicate that the current value should be serialized and
/// deserialized in Compact format. Reference form.
#[derive(Debug, Eq, PartialEq)]
pub struct CompactRef<'a, T>(pub &'a T);

/// Indicate that the current value should be serialized and
/// deserialized in Compact format. Value form.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Compact<T>(pub T);

impl<T> From<T> for Compact<T> {
	fn from(t: T) -> Self {
		Self(t)
	}
}

/// Calculate a ssz merkle tree root, dismissing the tree.
pub fn tree_root<D, T>(value: &T) -> H256 where
	T: IntoTree,
	D: Digest<OutputSize=U32>,
{
	value.into_tree(&mut NoopBackend::<DigestConstruct<D>>::default())
		.map(|ret| H256::from_slice(ret.as_ref()))
		.expect("Noop backend never fails in set; qed")
}
