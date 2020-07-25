use crate::{Backend, ReadBackend, WriteBackend, Construct};
use core::hash::Hash;
use core::ops::Deref;
use core::fmt;
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::collections::{HashMap as Map, HashSet as Set};
#[cfg(not(feature = "std"))]
use alloc::collections::{BTreeMap as Map, BTreeSet as Set};

/// Proving state.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ProvingState<V: Eq + Hash + Ord> {
	/// Proofs required for operations.
	pub proofs: Map<V, (V, V)>,
	/// Inserts of operations, which do not go into the proof.
	pub inserts: Set<V>,
}

impl<V: Eq + Hash + Ord> Default for ProvingState<V> {
	fn default() -> Self {
		Self {
			proofs: Default::default(),
			inserts: Default::default(),
		}
	}
}

impl<V: Eq + Hash + Ord> From<ProvingState<V>> for Proofs<V> {
	fn from(state: ProvingState<V>) -> Self {
		Self(state.proofs)
	}
}

/// Proving merkle database.
pub struct ProvingBackend<'a, DB: Backend + ?Sized> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord
{
	db: &'a mut DB,
	state: ProvingState<<DB::Construct as Construct>::Value>,
}

impl<'a, DB: Backend + ?Sized> ProvingBackend<'a, DB> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
	/// Create a new proving database.
	pub fn new(db: &'a mut DB) -> Self {
		Self {
			db,
			state: Default::default(),
		}
	}

	/// From proving state.
	pub fn from_state(state: ProvingState<<DB::Construct as Construct>::Value>, db: &'a mut DB) -> Self {
		Self { db, state }
	}

	/// Into proving state.
	pub fn into_state(self) -> ProvingState<<DB::Construct as Construct>::Value> {
		self.state
	}
}

impl<'a, DB: Backend + ?Sized> From<ProvingBackend<'a, DB>> for Proofs<<DB::Construct as Construct>::Value> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
	fn from(backend: ProvingBackend<'a, DB>) -> Self {
		backend.state.into()
	}
}

impl<'a, DB: Backend + ?Sized> Backend for ProvingBackend<'a, DB> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
	type Construct = DB::Construct;
	type Error = DB::Error;
}

impl<'a, DB: ReadBackend + ?Sized> ReadBackend for ProvingBackend<'a, DB> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
	fn get(
		&mut self,
		key: &<DB::Construct as Construct>::Value
	) -> Result<Option<(<DB::Construct as Construct>::Value, <DB::Construct as Construct>::Value)>, Self::Error> {
		let value = match self.db.get(key)? {
			Some(value) => value,
			None => return Ok(None),
		};
		if !self.state.inserts.contains(key) {
			self.state.proofs.insert(key.clone(), value.clone());
		}
		Ok(Some(value))
	}
}

impl<'a, DB: WriteBackend + ?Sized> WriteBackend for ProvingBackend<'a, DB> where
	<DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
	fn rootify(&mut self, key: &<DB::Construct as Construct>::Value) -> Result<(), Self::Error> {
		self.db.rootify(key)
	}

	fn unrootify(&mut self, key: &<DB::Construct as Construct>::Value) -> Result<(), Self::Error> {
		self.db.unrootify(key)
	}

	fn insert(
		&mut self,
		key: <DB::Construct as Construct>::Value,
		value: (<DB::Construct as Construct>::Value, <DB::Construct as Construct>::Value)
	) -> Result<(), Self::Error> {
		self.state.inserts.insert(key.clone());
		self.db.insert(key, value)
	}
}

/// Type of proofs.
pub struct Proofs<V>(Map<V, (V, V)>);

impl<V> Into<Map<V, (V, V)>> for Proofs<V> {
	fn into(self) -> Map<V, (V, V)> {
		self.0
	}
}

impl<V: Eq + Hash + Ord> Default for Proofs<V> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<V: Clone> Clone for Proofs<V> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<V> Deref for Proofs<V> {
	type Target = Map<V, (V, V)>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<V: Eq + Hash + Ord> PartialEq for Proofs<V> {
	fn eq(&self, other: &Self) -> bool {
		self.0.eq(&other.0)
	}
}

impl<V: Eq + Hash + Ord> Eq for Proofs<V>  { }

impl<V: Eq + Hash + Ord + fmt::Debug> fmt::Debug for Proofs<V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl<V: Eq + Hash + Ord + Clone + Default> Proofs<V> {
	/// Create compact merkle proofs from complete entries.
	pub fn into_compact(&self, root: V) -> CompactValue<V> {
		if let Some((left, right)) = self.0.get(&root) {
			let compact_left = self.into_compact(left.clone());
			let compact_right = self.into_compact(right.clone());
			CompactValue::Combined(Box::new((compact_left, compact_right)))
		} else {
			CompactValue::Single(root)
		}
	}

	/// Convert the compact value into full proofs.
	pub fn from_compact<C: Construct<Value=V>>(compact: CompactValue<V>) -> (Self, V) {
		match compact {
			CompactValue::Single(root) => (Proofs(Default::default()), root),
			CompactValue::Combined(boxed) => {
				let (compact_left, compact_right) = *boxed;
				let (left_proofs, left) = Self::from_compact::<C>(compact_left);
				let (right_proofs, right) = Self::from_compact::<C>(compact_right);
				let mut proofs = left_proofs.0.into_iter()
					.chain(right_proofs.0.into_iter())
					.collect::<Map<V, (V, V)>>();
				let key = C::intermediate_of(&left, &right);
				proofs.insert(key.clone(), (left, right));
				(Proofs(proofs), key)
			},
		}
	}
}

/// Compact proofs.
#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-codec", derive(parity_codec::Encode, parity_codec::Decode))]
pub enum CompactValue<V> {
	/// Single compact value.
	Single(V),
	/// Value is combined by other left and right entries.
	Combined(Box<(CompactValue<V>, CompactValue<V>)>),
}

impl<V: Default> Default for CompactValue<V> {
	fn default() -> Self {
		CompactValue::Single(Default::default())
	}
}

impl<V> CompactValue<V> {
	/// Get the length of the current value.
	pub fn len(&self) -> usize {
		match self {
			CompactValue::Single(_) => 1,
			CompactValue::Combined(boxed) => {
				boxed.as_ref().0.len() + boxed.as_ref().1.len()
			},
		}
	}
}
