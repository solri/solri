use bm::{ReadBackend, WriteBackend, Construct, Error, DanglingPackedVector, DanglingVector, Leak, Sequence};
use bm::utils::{vector_tree, host_len};
use primitive_types::{H256, U256};
use generic_array::GenericArray;
use alloc::vec::Vec;

use crate::{IntoTree, FromTree, Value, CompatibleConstruct};

/// Traits for vector converting into a composite tree structure.
pub trait IntoCompositeVectorTree {
	/// Convert this vector into merkle tree, writing nodes into the
	/// given database, and using the maximum length specified.
	fn into_composite_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for vector converting into a compact tree structure.
pub trait IntoCompactVectorTree {
	/// Convert this vector into merkle tree, writing nodes into the
	/// given database, and using the maximum length specified.
	fn into_compact_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for vector converting from a composite tree structure.
pub trait FromCompositeVectorTree: Sized {
	/// Convert this type from merkle tree, reading nodes from the
	/// given database, with given length and maximum length.
	fn from_composite_vector_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		len: usize,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

/// Traits for vector converting from a compact tree structure.
pub trait FromCompactVectorTree: Sized {
	/// Convert this type from merkle tree, reading nodes from the
	/// given database, with given length and maximum length.
	fn from_compact_vector_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		len: usize,
		max_len: Option<usize>,
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct;
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Elemental `Vec` reference. In ssz's definition, this is a basic "vector".
pub struct ElementalFixedVecRef<'a, T>(pub &'a [T]);
#[derive(Debug, Clone, Eq, PartialEq)]
/// Elemental `Vec` value. In ssz's definition, this is a basic "vector".
pub struct ElementalFixedVec<T>(pub Vec<T>);

macro_rules! impl_builtin_fixed_uint_vector {
	( $t:ty, $lt:ty ) => {
		impl<'a> IntoCompactVectorTree for ElementalFixedVecRef<'a, $t> {
			fn into_compact_vector_tree<DB: WriteBackend>(
				&self,
				db: &mut DB,
				max_len: Option<usize>
			) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
				DB::Construct: CompatibleConstruct,
			{
				let mut chunks: Vec<Vec<u8>> = Vec::new();

				for value in self.0 {
					if chunks.last().map(|v| v.len() == 32).unwrap_or(true) {
						chunks.push(Vec::new());
					}

					let current = chunks.last_mut().expect("chunks must have at least one item; qed");
					current.append(&mut value.to_le_bytes().into_iter().cloned().collect::<Vec<u8>>());
				}

				if let Some(last) = chunks.last_mut() {
					while last.len() < 32 {
						last.push(0u8);
					}
				}

				vector_tree(&chunks.into_iter().map(|c| {
					Value(H256::from_slice(&c))
				}).collect::<Vec<_>>(), db, max_len.map(|max| host_len::<typenum::U32, $lt>(max)))
			}
		}

		impl FromCompactVectorTree for ElementalFixedVec<$t> {
			fn from_compact_vector_tree<DB: ReadBackend>(
				root: &<DB::Construct as Construct>::Value,
				db: &mut DB,
				len: usize,
				max_len: Option<usize>
			) -> Result<Self, Error<DB::Error>> where
				DB::Construct: CompatibleConstruct,
			{
				let packed = DanglingPackedVector::<DB::Construct, GenericArray<u8, $lt>, typenum::U32, $lt>::from_leaked(
					(root.clone(), len, max_len)
				);

				let mut ret = Vec::new();
				for i in 0..len {
					let value = packed.get(db, i)?;
					let mut bytes = <$t>::default().to_le_bytes();
					bytes.copy_from_slice(value.as_slice());
					ret.push(<$t>::from_le_bytes(bytes));
				}

				Ok(Self(ret))
			}
		}
	}
}

impl_builtin_fixed_uint_vector!(u8, typenum::U1);
impl_builtin_fixed_uint_vector!(u16, typenum::U2);
impl_builtin_fixed_uint_vector!(u32, typenum::U4);
impl_builtin_fixed_uint_vector!(u64, typenum::U8);
impl_builtin_fixed_uint_vector!(u128, typenum::U16);

impl<'a> IntoCompactVectorTree for ElementalFixedVecRef<'a, U256> {
	fn into_compact_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		vector_tree(&self.0.iter().map(|uint| {
			let mut ret = Value::default();
			uint.to_little_endian(&mut ret.0.as_mut());
			ret
		}).collect::<Vec<_>>(), db, max_len)
	}
}

impl FromCompactVectorTree for ElementalFixedVec<U256> {
	fn from_compact_vector_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		len: usize,
		max_len: Option<usize>
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let vector = DanglingVector::<DB::Construct>::from_leaked(
			(root.clone(), len, max_len)
		);

		let mut ret = Vec::new();
		for i in 0..len {
			let value = vector.get(db, i)?;
			ret.push(U256::from(value.as_ref()));
		}

		Ok(Self(ret))
	}
}

impl<'a> IntoCompactVectorTree for ElementalFixedVecRef<'a, bool> {
	fn into_compact_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let mut bytes = Vec::new();
		bytes.resize((self.0.len() + 7) / 8, 0u8);

		for i in 0..self.0.len() {
			bytes[i / 8] |= (self.0[i] as u8) << (i % 8);
		}

		ElementalFixedVecRef(&bytes).into_compact_vector_tree(db, max_len.map(|l| {
			(l + 7) / 8
		}))
	}
}

impl FromCompactVectorTree for ElementalFixedVec<bool> {
	fn from_compact_vector_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		len: usize,
		max_len: Option<usize>
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let packed = DanglingPackedVector::<DB::Construct, GenericArray<u8, typenum::U1>, typenum::U32, typenum::U1>::from_leaked(
			(root.clone(), (len + 7) / 8, max_len.map(|l| (l + 7) / 8))
		);

		let mut bytes = Vec::new();
		for i in 0..packed.len() {
			bytes.push(packed.get(db, i)?[0]);
		}
		let mut ret = Vec::new();
		for i in 0..len {
			ret.push(bytes[i / 8] & (1 << (i % 8)) != 0);
		}
		// TODO: check to make sure rest of the bits are unset.

		Ok(Self(ret))
	}
}

impl<'a, T> IntoCompositeVectorTree for ElementalFixedVecRef<'a, T> where
	T: IntoTree,
{
	fn into_composite_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		vector_tree(&self.0.iter().map(|value| {
			value.into_tree(db)
		}).collect::<Result<Vec<_>, _>>()?, db, max_len)
	}
}

fn from_composite_vector_tree<T, F, DB: ReadBackend>(
	root: &<DB::Construct as Construct>::Value,
	db: &mut DB,
	len: usize,
	max_len: Option<usize>,
	f: F
) -> Result<ElementalFixedVec<T>, Error<DB::Error>> where
	DB::Construct: CompatibleConstruct,
	F: Fn(&<DB::Construct as Construct>::Value, &mut DB) -> Result<T, Error<DB::Error>>
{
	let vector = DanglingVector::<DB::Construct>::from_leaked(
		(root.clone(), len, max_len)
	);
	let mut ret = Vec::new();

	for i in 0..len {
		let value = vector.get(db, i)?;
		ret.push(f(&value, db)?);
	}

	Ok(ElementalFixedVec(ret))
}

impl<T: FromTree> FromCompositeVectorTree for ElementalFixedVec<T> {
	fn from_composite_vector_tree<DB: ReadBackend>(
		root: &<DB::Construct as Construct>::Value,
		db: &mut DB,
		len: usize,
		max_len: Option<usize>
	) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		from_composite_vector_tree(root, db, len, max_len, |value, db| T::from_tree(value, db))
	}
}

impl<T> IntoCompactVectorTree for ElementalFixedVec<T> where
	for<'a> ElementalFixedVecRef<'a, T>: IntoCompactVectorTree,
{
	fn into_compact_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, max_len)
	}
}

impl<T> IntoCompositeVectorTree for ElementalFixedVec<T> where
	for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree,
{
	fn into_composite_vector_tree<DB: WriteBackend>(
		&self,
		db: &mut DB,
		max_len: Option<usize>
	) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalFixedVecRef(&self.0).into_composite_vector_tree(db, max_len)
	}
}
