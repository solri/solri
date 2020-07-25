use alloc::collections::btree_map::BTreeMap;
use crate::{RevDB, Revision};

/// Memory revision DB error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryRevDBError {
	/// Revert target out of range.
	InvalidRevertTarget,
	/// Revision out of range.
	NoRevision,
}

/// In-memory revision database.
#[derive(Clone, Debug)]
pub struct MemoryRevDB<K, V> {
	db: BTreeMap<K, BTreeMap<Revision, Option<V>>>,
	journal: BTreeMap<Revision, Vec<K>>,
	revision: Revision,
}

impl<K: Ord, V> MemoryRevDB<K, V> {
	pub fn new() -> Self {
		Self {
			db: Default::default(),
			journal: Default::default(),
			revision: 0,
		}
	}
}

impl<K: Ord + Clone, V: Clone> RevDB for MemoryRevDB<K, V> {
	type Key = K;
	type Value = Option<V>;
	type Error = MemoryRevDBError;

	fn revision(&self) -> Revision {
		self.revision
	}

    fn revert_to(&mut self, target: Revision) -> Result<(), Self::Error> {
		if target > self.revision {
			return Err(MemoryRevDBError::InvalidRevertTarget)
		}

		let mut current = self.revision;
		while current > target {
			let keys = self.journal.remove(&current).unwrap_or_default();
			for key in keys {
				let to_remove = self.db.get_mut(&key).map(|rvalues| {
					rvalues.remove(&current);
					rvalues.len() == 0
				}).unwrap_or(false);
				if to_remove {
					self.db.remove(&key);
				}
			}
			current -= 1;
		}

		self.revision = target;

		Ok(())
	}

    fn get(&self, target: Revision, key: &Self::Key) -> Result<Self::Value, Self::Error> {
		if target > self.revision {
			return Err(MemoryRevDBError::NoRevision)
		}

		if let Some(rvalues) = self.db.get(key) {
			for (revision, value) in rvalues.iter().rev() {
				if *revision <= target {
					return Ok(value.clone())
				}
			}
		}

		Ok(None)
	}

    fn commit(
        &mut self,
        values: impl IntoIterator<Item=(Self::Key, Self::Value)>
    ) -> Result<Revision, Self::Error> {
		let new = self.revision + 1;

		let mut keys = Vec::new();
		for (key, value) in values {
			self.db.entry(key.clone()).or_default().insert(new, value);
			keys.push(key);
		}

		self.journal.insert(new, keys);
		self.revision = new;

		Ok(new)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn should_handle_commit_revert() {
		let mut db = MemoryRevDB::<u64, u64>::new();

		assert_eq!(db.revision(), 0);
		assert_eq!(db.get(0, &1), Ok(None));

		db.commit(vec![(1, Some(5))]).unwrap();
		db.commit(vec![(1, Some(7))]).unwrap();
		db.commit(vec![(1, None)]).unwrap();
		db.commit(vec![(1, Some(9))]).unwrap();

		assert_eq!(db.revision(), 4);
		assert_eq!(db.get(1, &1), Ok(Some(5)));
		assert_eq!(db.get(2, &1), Ok(Some(7)));
		assert_eq!(db.get(3, &1), Ok(None));
		assert_eq!(db.get(4, &1), Ok(Some(9)));

		db.revert_to(2).unwrap();
		assert_eq!(db.get(1, &1), Ok(Some(5)));
		assert_eq!(db.get(2, &1), Ok(Some(7)));
		assert_eq!(db.get(3, &1), Err(MemoryRevDBError::NoRevision));
		assert_eq!(db.get(4, &1), Err(MemoryRevDBError::NoRevision));
	}
}
