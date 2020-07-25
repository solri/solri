use typenum::Unsigned;
use bm::{Error, Construct, ReadBackend, WriteBackend};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use alloc::vec::Vec;
use crate::{ElementalVariableVecRef, ElementalVariableVec,
			IntoTree, IntoCompactListTree, IntoCompositeListTree,
			FromTree, FromCompactListTree, FromCompositeListTree,
			Compact, CompactRef, CompatibleConstruct};

/// Vec value with maximum length.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MaxVec<T, ML>(pub Vec<T>, PhantomData<ML>);

impl<T, ML> Deref for MaxVec<T, ML> {
	type Target = Vec<T>;

	fn deref(&self) -> &Vec<T> {
		&self.0
	}
}

impl<T, ML> DerefMut for MaxVec<T, ML> {
	fn deref_mut(&mut self) -> &mut Vec<T> {
		&mut self.0
	}
}

impl<T, ML> AsRef<[T]> for MaxVec<T, ML> {
	fn as_ref(&self) -> &[T] {
		&self.0
	}
}

impl<T, ML> Default for MaxVec<T, ML> {
	fn default() -> Self {
		Self(Vec::new(), PhantomData)
	}
}

impl<T, ML> From<Vec<T>> for MaxVec<T, ML> {
	fn from(vec: Vec<T>) -> Self {
		Self(vec, PhantomData)
	}
}

impl<T, ML> Into<Vec<T>> for MaxVec<T, ML> {
	fn into(self) -> Vec<T> {
		self.0
	}
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize, N: Unsigned> serde::Serialize for MaxVec<T, N> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>, N: Unsigned> serde::Deserialize<'de> for MaxVec<T, N> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
		D: serde::Deserializer<'de>,
	{
		let vec = Vec::<T>::deserialize(deserializer)?;
		if vec.len() > N::to_usize() {
			return Err(<D::Error as serde::de::Error>::custom("invalid length"))
		}

		Ok(Self(vec, PhantomData))
	}
}

#[cfg(feature = "parity-codec")]
impl<T: parity_codec::Encode, N: Unsigned> parity_codec::Encode for MaxVec<T, N> {
	fn encode_to<W: parity_codec::Output>(&self, dest: &mut W) {
		self.0.encode_to(dest)
	}
}

#[cfg(feature = "parity-codec")]
impl<T: parity_codec::Decode, N: Unsigned> parity_codec::Decode for MaxVec<T, N> {
	fn decode<I: parity_codec::Input>(input: &mut I) -> Option<Self> {
		let decoded = Vec::<T>::decode(input)?;
		if decoded.len() <= N::to_usize() {
			Some(Self(decoded, PhantomData))
		} else {
			None
		}
	}
}

impl<T, ML: Unsigned> IntoTree for MaxVec<T, ML> where
	for<'b> ElementalVariableVecRef<'b, T>: IntoCompositeListTree,
{
	fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self.0).into_composite_list_tree(db, Some(ML::to_usize()))
	}
}

impl<T, ML: Unsigned> FromTree for MaxVec<T, ML> where
	for<'a> ElementalVariableVec<T>: FromCompositeListTree,
{
	fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let value = ElementalVariableVec::<T>::from_composite_list_tree(
			root, db, Some(ML::to_usize())
		)?;
		Ok(MaxVec(value.0, PhantomData))
	}
}

impl<'a, T, ML: Unsigned> IntoTree for CompactRef<'a, MaxVec<T, ML>> where
	for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree,
{
	fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
	}
}

impl<T, ML: Unsigned> IntoTree for Compact<MaxVec<T, ML>> where
	for<'b> ElementalVariableVecRef<'b, T>: IntoCompactListTree,
{
	fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self.0).into_compact_list_tree(db, Some(ML::to_usize()))
	}
}

impl<T, ML: Unsigned> FromTree for Compact<MaxVec<T, ML>> where
	for<'a> ElementalVariableVec<T>: FromCompactListTree,
{
	fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		let value = ElementalVariableVec::<T>::from_compact_list_tree(
			root, db, Some(ML::to_usize())
		)?;
		Ok(Self(MaxVec(value.0, PhantomData)))
	}
}

impl<T> IntoTree for [T] where
	for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree,
{
	fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
	}
}

impl<T> IntoTree for Vec<T> where
	for<'a> ElementalVariableVecRef<'a, T>: IntoCompositeListTree,
{
	fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVecRef(&self).into_composite_list_tree(db, None)
	}
}

impl<T> FromTree for Vec<T> where
	ElementalVariableVec<T>: FromCompositeListTree,
{
	fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
		DB::Construct: CompatibleConstruct,
	{
		ElementalVariableVec::from_composite_list_tree(root, db, None).map(|ret| ret.0)
	}
}
