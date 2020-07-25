#[cfg(feature = "std")]
use std::collections::HashMap as Map;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
use generic_array::GenericArray;
use digest::Digest;
use core::marker::PhantomData;
use core::hash::Hash;

use crate::{Construct, Backend, ReadBackend, WriteBackend};

/// Empty status.
pub trait EmptyStatus {
	/// Is the backend using unit empty.
	fn is_unit() -> bool { !Self::is_inherited() }
	/// Is the backend using inherited empty.
	fn is_inherited() -> bool { !Self::is_unit() }
}

/// Inherited empty.
pub struct InheritedEmpty;

impl EmptyStatus for InheritedEmpty {
	fn is_inherited() -> bool { true }
}

/// Unit empty.
pub struct UnitEmpty;

impl EmptyStatus for UnitEmpty {
	fn is_unit() -> bool { true }
}

/// Unit Digest construct.
pub struct UnitDigestConstruct<D: Digest, V=GenericArray<u8, <D as Digest>::OutputSize>>(PhantomData<(D, V)>);

impl<D: Digest, V> Construct for UnitDigestConstruct<D, V> where
	V: From<GenericArray<u8, D::OutputSize>> + AsRef<[u8]> + Default + Clone,
{
	type Value = V;

	fn intermediate_of(left: &Self::Value, right: &Self::Value) -> Self::Value {
		let mut digest = D::new();
		digest.input(&left.as_ref()[..]);
		digest.input(&right.as_ref()[..]);
		digest.result().into()
	}

	fn empty_at<DB: WriteBackend<Construct=Self> + ?Sized>(
		_db: &mut DB,
		_depth_to_bottom: usize
	) -> Result<Self::Value, DB::Error> {
		Ok(Default::default())
	}
}

/// Inherited Digest construct.
pub struct InheritedDigestConstruct<D: Digest, V=GenericArray<u8, <D as Digest>::OutputSize>>(PhantomData<(D, V)>);

impl<D: Digest, V> Construct for InheritedDigestConstruct<D, V> where
	V: From<GenericArray<u8, D::OutputSize>> + AsRef<[u8]> + Default + Clone,
{
	type Value = V;

	fn intermediate_of(left: &Self::Value, right: &Self::Value) -> Self::Value {
		let mut digest = D::new();
		digest.input(&left.as_ref()[..]);
		digest.input(&right.as_ref()[..]);
		digest.result().into()
	}

	fn empty_at<DB: WriteBackend<Construct=Self> + ?Sized>(
		db: &mut DB,
		depth_to_bottom: usize
	) -> Result<Self::Value, DB::Error> {
		let mut current = Self::Value::default();
		for _ in 0..depth_to_bottom {
			let value = (current.clone(), current);
			let key = Self::intermediate_of(&value.0, &value.1);
			db.insert(key.clone(), value)?;
			current = key;
		}
		Ok(current)
	}
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// Noop DB error.
pub enum NoopBackendError {
	/// Not supported get operation.
	NotSupported,
}

/// Noop merkle database.
pub struct NoopBackend<C: Construct>(
	PhantomData<C>,
);

impl<C: Construct> Default for NoopBackend<C> where
	C::Value: Eq + Hash + Ord
{
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<C: Construct> Clone for NoopBackend<C> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<C: Construct> Backend for NoopBackend<C> {
	type Construct = C;
	type Error = NoopBackendError;
}

impl<C: Construct> ReadBackend for NoopBackend<C> {
	fn get(
		&mut self,
		_key: &C::Value,
	) -> Result<Option<(C::Value, C::Value)>, Self::Error> {
		Err(NoopBackendError::NotSupported)
	}
}

impl<C: Construct> WriteBackend for NoopBackend<C> {
	fn rootify(&mut self, _key: &C::Value) -> Result<(), Self::Error> {
		Ok(())
	}

	fn unrootify(&mut self, _key: &C::Value) -> Result<(), Self::Error> {
		Ok(())
	}

	fn insert(
		&mut self,
		_key: C::Value,
		_value: (C::Value, C::Value)
	) -> Result<(), Self::Error> {
		Ok(())
	}
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// In-memory DB error.
pub enum InMemoryBackendError {
	/// Fetching key not exist.
	FetchingKeyNotExist,
	/// Trying to rootify a non-existing key.
	RootifyKeyNotExist,
	/// Set subkey does not exist.
	SetIntermediateNotExist
}

#[cfg(feature = "std")]
impl std::fmt::Display for InMemoryBackendError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}

#[cfg(feature = "std")]
impl std::error::Error for InMemoryBackendError { }

/// In-memory merkle database.
pub struct InMemoryBackend<C: Construct>(
	Map<C::Value, (Option<(C::Value, C::Value)>, Option<usize>)>,
);

impl<C: Construct> Default for InMemoryBackend<C> where
	C::Value: Eq + Hash + Ord
{
	fn default() -> Self {
		let mut map = Map::default();
		map.insert(Default::default(), (None, None));

		Self(map)
	}
}

impl<C: Construct> Clone for InMemoryBackend<C> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<C: Construct> InMemoryBackend<C> where
	C::Value: Eq + Hash + Ord,
{
	fn remove(&mut self, old_key: &C::Value) -> Result<(), InMemoryBackendError> {
		let (old_value, to_remove) = {
			let value = match self.0.get_mut(old_key) {
				Some(value) => value,
				None => return Ok(()),
			};
			value.1.as_mut().map(|v| *v -= 1);
			(value.0.clone(), value.1.map(|v| v == 0).unwrap_or(false))
		};

		if to_remove {
			if let Some(old_value) = old_value {
				self.remove(&old_value.0)?;
				self.remove(&old_value.1)?;
			}

			self.0.remove(old_key);
		}

		Ok(())
	}

	/// Populate the database with proofs.
	pub fn populate(&mut self, proofs: Map<C::Value, (C::Value, C::Value)>) {
		for (key, (left, right)) in proofs {
			self.0.insert(key, (Some((left.clone(), right.clone())), None));
			self.0.entry(left).or_insert((None, None));
			self.0.entry(right).or_insert((None, None));
		}
	}
}

impl<C: Construct> AsRef<Map<C::Value, (Option<(C::Value, C::Value)>, Option<usize>)>> for InMemoryBackend<C> {
	fn as_ref(&self) -> &Map<C::Value, (Option<(C::Value, C::Value)>, Option<usize>)> {
		&self.0
	}
}

impl<C: Construct> Backend for InMemoryBackend<C> {
	type Construct = C;
	type Error = InMemoryBackendError;
}

impl<C: Construct> ReadBackend for InMemoryBackend<C> where
	C::Value: Eq + Hash + Ord,
{
	fn get(&mut self, key: &C::Value) -> Result<Option<(C::Value, C::Value)>, Self::Error> {
		Ok(self.0.get(key).map(|v| v.0.clone()).unwrap_or(None))
	}
}

impl<C: Construct> WriteBackend for InMemoryBackend<C> where
	C::Value: Eq + Hash + Ord,
{
	fn rootify(&mut self, key: &C::Value) -> Result<(), Self::Error> {
		self.0.entry(key.clone()).or_insert((None, Some(0))).1
			.as_mut().map(|v| *v += 1);
		Ok(())
	}

	fn unrootify(&mut self, key: &C::Value) -> Result<(), Self::Error> {
		self.remove(key)?;
		Ok(())
	}

	fn insert(
		&mut self,
		key: C::Value,
		value: (C::Value, C::Value)
	) -> Result<(), Self::Error> {
		if self.0.contains_key(&key) {
			return Ok(())
		}

		let (left, right) = value;

		self.0.entry(left.clone()).or_insert((None, Some(0))).1
			.as_mut().map(|v| *v += 1);
		self.0.entry(right.clone()).or_insert((None, Some(0))).1
			.as_mut().map(|v| *v += 1);

		self.0.insert(key, (Some((left, right)), Some(0)));
		Ok(())
	}
}
