use bm::{Error, ReadBackend, WriteBackend, Construct};
use primitive_types::U256;
use alloc::vec::Vec;

use crate::{ElementalFixedVec, FromCompactVectorTree, FromCompositeVectorTree,
			ElementalFixedVecRef, IntoCompactVectorTree,
			IntoCompositeVectorTree, CompatibleConstruct};
use crate::utils::{mix_in_length, decode_with_length};

/// Traits for list converting into a tree structure.
pub trait IntoCompositeListTree {
	/// Convert this list into merkle tree, writing nodes into the
	/// given database, and using the maximum length specified.
	fn into_composite_list_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for list converting into a tree structure.
pub trait IntoCompactListTree {
	/// Convert this list into merkle tree, writing nodes into the
	/// given database, and using the maximum length specified.
	fn into_compact_list_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for list converting from a tree structure.
pub trait FromCompositeListTree: Sized {
	/// Convert this type from merkle tree, reading nodes from the
	/// given database, with given maximum length.
	fn from_composite_list_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for list converting from a tree structure.
pub trait FromCompactListTree: Sized {
	/// Convert this type from merkle tree, reading nodes from the
	/// given database, with given maximum length.
	fn from_compact_list_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` reference. In `ssz`'s definition, this is a "list".
pub struct ElementalVariableVecRef<'a, T>(pub &'a [T]);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Variable `Vec` value. In `ssz`'s definition, this is a "list".
pub struct ElementalVariableVec<T>(pub Vec<T>);

macro_rules! impl_packed {
	( $t:ty ) => {
		impl<'a> IntoCompactListTree for ElementalVariableVecRef<'a, $t> {
			fn into_compact_list_tree<DB: WriteBackend>(
				&self,
				db: &mut DB,
				max_len: Option<usize>
			) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
				DB::Construct: CompatibleConstruct,
			{
				let len = self.0.len();

				mix_in_length(
					&ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, max_len)?,
					db,
					len,
				)
			}
		}
	}
}

impl_packed!(bool);
impl_packed!(u8);
impl_packed!(u16);
impl_packed!(u32);
impl_packed!(u64);
impl_packed!(u128);
impl_packed!(U256);

impl<'a, T> IntoCompositeListTree for ElementalVariableVecRef<'a, T> where
	for<'b> ElementalFixedVecRef<'b, T>: IntoCompositeVectorTree,
{
	fn into_composite_list_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let len = self.0.len();

		mix_in_length(
			&ElementalFixedVecRef(&self.0).into_composite_vector_tree(db, max_len)?,
			db,
			len,
		)
	}
}

fn from_list_tree<T, F, DB: ReadBackend>(
	root: &<DB::Construct as Construct>::Value,
	db: &mut DB,
	max_len: Option<usize>,
	f: F
) -> Result<ElementalVariableVec<T>, Error<DB::Error>> where
	DB::Construct: CompatibleConstruct,
	F: FnOnce(&<DB::Construct as Construct>::Value, &mut DB, usize, Option<usize>) -> Result<ElementalFixedVec<T>, Error<DB::Error>>
{
	let (vector_root, len) = decode_with_length::<<DB::Construct as Construct>::Value, _>(root, db)?;

	let vector = f(
		&vector_root, db, len, max_len
	)?;

	Ok(ElementalVariableVec(vector.0))
}

impl<T> FromCompactListTree for ElementalVariableVec<T> where
	ElementalFixedVec<T>: FromCompactVectorTree,
{
	fn from_compact_list_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
			ElementalFixedVec::<T>::from_compact_vector_tree(
				vector_root, db, len, max_len
			)
		})
	}
}

impl<T> FromCompositeListTree for ElementalVariableVec<T> where
	ElementalFixedVec<T>: FromCompositeVectorTree,
{
	fn from_composite_list_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		from_list_tree(root, db, max_len, |vector_root, db, len, max_len| {
			ElementalFixedVec::<T>::from_composite_vector_tree(
				vector_root, db, len, max_len
			)
		})
	}
}

impl<T> IntoCompactListTree for ElementalVariableVec<T> where
	for<'a> ElementalVariableVecRef<'a, T>: IntoCompactListTree,
{
	fn into_compact_list_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self.0).into_compact_list_tree(db, max_len)
	}
}

impl<T> IntoCompositeListTree for ElementalVariableVec<T> where
	for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree,
{
	fn into_composite_list_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self.0).into_composite_list_tree(db, max_len)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{IntoTree, FromTree, DigestConstruct};

	use bm::InMemoryBackend;
	use sha2::Sha256;

	#[test]
	fn test_plain() {
		let data = {
			let mut ret = Vec::new();
			for i in 0..17u16 {
				ret.push(i);
			}
			ret
		};

		let mut db = InMemoryBackend::<DigestConstruct<Sha256>>::default();
		let encoded = data.into_tree(&mut db).unwrap();
		let decoded = Vec::<u16>::from_tree(&encoded, &mut db).unwrap();
		assert_eq!(data, decoded);
	}
}
