use core::marker::PhantomData;
use alloc::vec::Vec;

use crate::index::{Index, IndexSelection, IndexRoute};
use crate::traits::{Construct, ReadBackend, WriteBackend,
					RootStatus, Owned, Dangling, Leak, Error, Tree};

/// `Raw` with owned root.
pub type OwnedRaw<C> = Raw<Owned, C>;

/// `Raw` with dangling root.
pub type DanglingRaw<C> = Raw<Dangling, C>;

/// Raw merkle tree.
pub struct Raw<R: RootStatus, C: Construct> {
	root: C::Value,
	_marker: PhantomData<(R, C)>,
}

impl<R: RootStatus, C: Construct> Default for Raw<R, C> {
	fn default() -> Self {
		Self {
			root: Default::default(),
			_marker: PhantomData,
		}
	}
}

impl<R: RootStatus, C: Construct> Tree for Raw<R, C> {
	type RootStatus = R;
	type Construct = C;

	fn root(&self) -> C::Value {
		self.root.clone()
	}

	fn drop<DB: WriteBackend<Construct=C> + ?Sized>(
		self,
		db: &mut DB
	) -> Result<(), Error<DB::Error>> {
		if R::is_owned() {
			db.unrootify(&self.root())?;
		}
		Ok(())
	}

	fn into_raw(self) -> Raw<R, C> {
		self
	}
}

impl<R: RootStatus, C: Construct> Raw<R, C> {
	/// Return a reference to a subtree.
	pub fn subtree<DB: ReadBackend<Construct=C> + ?Sized>(
		&self,
		db: &mut DB,
		index: Index
	) -> Result<DanglingRaw<C>, Error<DB::Error>> {
		let subroot = self.get(db, index)?.ok_or(Error::CorruptedDatabase)?;
		Ok(Raw {
			root: subroot,
			_marker: PhantomData,
		})
	}

	/// Get value from the tree via generalized merkle index.
	pub fn get<DB: ReadBackend<Construct=C> + ?Sized>(
		&self,
		db: &mut DB,
		index: Index
	) -> Result<Option<C::Value>, Error<DB::Error>> {
		match index.route() {
			IndexRoute::Root => Ok(Some(self.root.clone())),
			IndexRoute::Select(selections) => {
				let mut current = self.root.clone();

				for selection in selections {
					let pair = match db.get(&current)? {
						Some(pair) => pair,
						None => return Ok(None),
					};
					current = match selection {
						IndexSelection::Left => pair.0.clone(),
						IndexSelection::Right => pair.1.clone(),
					};
				}

				Ok(Some(current))
			},
		}
	}

	/// Set value of the merkle tree via generalized merkle index.
	pub fn set<DB: WriteBackend<Construct=C> + ?Sized>(
		&mut self,
		db: &mut DB,
		index: Index,
		set: C::Value,
	) -> Result<(), Error<DB::Error>> {
		let route = index.route();
		let mut values = {
			let mut values = Vec::new();
			let mut depth = 1;
			let mut current = Some(self.root.clone());

			loop {
				let sel = match route.at_depth(depth) {
					Some(sel) => sel,
					None => break,
				};
				match current.clone() {
					Some(cur) => {
						let value = db.get(&cur)?;
						match value {
							Some((left, right)) => {
								values.push((sel, (left.clone(), right.clone())));
								current = Some(match sel {
									IndexSelection::Left => left,
									IndexSelection::Right => right,
								});
							},
							None => {
								values.push((sel, Default::default()));
							},
						}
					},
					None => {
						values.push((sel, Default::default()));
					},
				}
				depth += 1;
			}

			values
		};

		let mut update = set;
		loop {
			let (sel, mut value) = match values.pop() {
				Some(v) => v,
				None => break,
			};

			match sel {
				IndexSelection::Left => { value.0 = update.clone(); }
				IndexSelection::Right => { value.1 = update.clone(); }
			}

			let intermediate = C::intermediate_of(&value.0, &value.1);

			db.insert(intermediate.clone(), value)?;
			update = intermediate;
		}


		if R::is_owned() {
			db.rootify(&update)?;
			db.unrootify(&self.root)?;
		}

		self.root = update;
		Ok(())
	}
}

impl<R: RootStatus, C: Construct> Leak for Raw<R, C> {
	type Metadata = C::Value;

	fn metadata(&self) -> Self::Metadata {
		self.root()
	}

	fn from_leaked(root: Self::Metadata) -> Self {
		Self {
			root,
			_marker: PhantomData,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::traits::Owned;
	use generic_array::{arr, arr_impl};
	use sha2::Sha256;

	type Construct = crate::InheritedDigestConstruct<Sha256>;
	type InMemory = crate::memory::InMemoryBackend<Construct>;

	#[test]
	fn test_merkle_selections() {
		assert_eq!(Index::root().route(), IndexRoute::Root);
		assert_eq!(
			Index::root().left().route(),
			IndexRoute::Select(vec![
				IndexSelection::Left
			])
		);
		assert_eq!(
			Index::root().left().right().route(),
			IndexRoute::Select(vec![
				IndexSelection::Left,
				IndexSelection::Right,
			])
		);
		assert_eq!(
			Index::root().right().left().left().route(),
			IndexRoute::Select(vec![
				IndexSelection::Right,
				IndexSelection::Left,
				IndexSelection::Left,
			])
		);
		assert_eq!(
			Index::root().left().left().right().left().route(),
			IndexRoute::Select(vec![
				IndexSelection::Left,
				IndexSelection::Left,
				IndexSelection::Right,
				IndexSelection::Left,
			])
		);
		assert_eq!(
			Index::root().left().right().right().left().right().route(),
			IndexRoute::Select(vec![
				IndexSelection::Left,
				IndexSelection::Right,
				IndexSelection::Right,
				IndexSelection::Left,
				IndexSelection::Right,
			])
		);
	}

	#[test]
	fn test_selection_at() {
		assert_eq!(Index::root().right().route().at_depth(1), Some(IndexSelection::Right));
	}

	#[test]
	fn test_set_empty() {
		let mut db = InMemory::default();
		let mut list = Raw::<Owned, Construct>::default();

		let mut last_root = list.root();
		for _ in 0..3 {
			list.set(&mut db, Index::from_one(2).unwrap(), last_root.clone()).unwrap();
			list.set(&mut db, Index::from_one(3).unwrap(), last_root.clone()).unwrap();
			last_root = list.root();
		}
	}

	macro_rules! sinarr {
		( $x:expr ) => (
			arr![
				u8;
				$x, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0
			]
		)
	}

	#[test]
	fn test_set_skip() {
		let mut db = InMemory::default();
		let mut list = Raw::<Owned, Construct>::default();

		list.set(&mut db, Index::from_one(4).unwrap(), sinarr!(2)).unwrap();
		assert_eq!(list.get(&mut db, Index::from_one(4).unwrap()).unwrap(), Some(sinarr!(2)));
		list.set(&mut db, Index::from_one(4).unwrap(), sinarr!(3)).unwrap();
		assert_eq!(list.get(&mut db, Index::from_one(4).unwrap()).unwrap(), Some(sinarr!(3)));
	}

	#[test]
	fn test_set_basic() {
		let mut db = InMemory::default();
		let mut list = Raw::<Owned, Construct>::default();

		for i in 4..8 {
			list.set(&mut db, Index::from_one(i).unwrap(), sinarr!(i as u8)).unwrap();
		}
	}

	#[test]
	fn test_set_only() {
		let mut db1 = InMemory::default();
		let mut db2 = InMemory::default();
		let mut list1 = Raw::<Owned, Construct>::default();
		let mut list2 = Raw::<Owned, Construct>::default();

		for i in 32..64 {
			list1.set(&mut db1, Index::from_one(i).unwrap(), sinarr!(i as u8)).unwrap();
		}
		for i in (32..64).rev() {
			list2.set(&mut db2, Index::from_one(i).unwrap(), sinarr!(i as u8)).unwrap();
		}
		assert_eq!(db1.as_ref(), db2.as_ref());
		for i in 32..64 {
			let val1 = list1.get(&mut db1, Index::from_one(i).unwrap()).unwrap().unwrap();
			let val2 = list2.get(&mut db2, Index::from_one(i).unwrap()).unwrap().unwrap();
			assert_eq!(val1, sinarr!(i as u8));
			assert_eq!(val2, sinarr!(i as u8));
		}

		list1.set(&mut db1, Index::from_one(1).unwrap(), sinarr!(1)).unwrap();
		assert_eq!(db1.as_ref().len(), 2);
	}

	#[test]
	fn test_intermediate() {
		let mut db = InMemory::default();
		let mut list = Raw::<Owned, Construct>::default();
		list.set(&mut db, Index::from_one(2).unwrap(), Default::default()).unwrap();
		assert_eq!(list.get(&mut db, Index::from_one(3).unwrap()).unwrap().unwrap(), Default::default());

		let empty1 = list.get(&mut db, Index::from_one(1).unwrap()).unwrap().unwrap();
		list.set(&mut db, Index::from_one(2).unwrap(), empty1.clone()).unwrap();
		list.set(&mut db, Index::from_one(3).unwrap(), empty1.clone()).unwrap();
		for i in 4..8 {
			assert_eq!(list.get(&mut db, Index::from_one(i).unwrap()).unwrap().unwrap(), Default::default());
		}
		assert_eq!(db.as_ref().len(), 3);

		let mut db1 = db.clone();
		let mut list1 = Raw::<Owned, Construct>::from_leaked(list.root());
		list.set(&mut db, Index::from_one(1).unwrap(), empty1.clone()).unwrap();
		assert_eq!(list.get(&mut db, Index::from_one(3).unwrap()).unwrap().unwrap(), Default::default());
		assert_eq!(db.as_ref().len(), 2);

		list1.set(&mut db1, Index::from_one(1).unwrap(), sinarr!(0)).unwrap();
		assert_eq!(list1.get(&mut db1, Index::from_one(1).unwrap()).unwrap().unwrap(), sinarr!(0));
		assert_eq!(db1.as_ref().len(), 1);
	}
}
