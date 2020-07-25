use alloc::collections::btree_map::BTreeMap;
use crate::{RevDB, Revision};

/// Memory revision DB error.
pub enum MemoryRevDBError {
	/// Revert target out of range.
	InvalidRevertTarget,
}

/// In-memory revision database.
pub struct MemoryRevDB<K, V> {
	db: BTreeMap<K, BTreeMap<Revision, Option<V>>>,
	journal: BTreeMap<Revision, Vec<K>>,
	revision: Revision,
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

		Ok(())
	}

    fn get(&self, target: Revision, key: &Self::Key) -> Result<Self::Value, Self::Error> {
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
        values: impl Iterator<Item=(Self::Key, Self::Value)>
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
