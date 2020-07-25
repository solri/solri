use rocksdb::DB;
use codec::{Encode, Decode};
use crate::{RevDB, Revision};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RocksRevDBError {
	/// Backend error.
	Backend(String),
	/// Invalid revision metadata in database.
	InvalidRevisionData,
	/// Invalid journal data.
	InvalidJournalData,
	/// Invalid data.
	InvalidData,
	/// Revert target out of range.
	InvalidRevertTarget,
	/// Invalid column family name.
	InvalidColumnFamily,
	/// Revision not found.
	NoRevision,
}

impl From<rocksdb::Error> for RocksRevDBError {
	fn from(err: rocksdb::Error) -> Self {
		Self::Backend(err.into_string())
	}
}

pub struct RocksRevDB {
	db: DB,
	data_cf: String,
	journal_cf: String,
	revision: Revision,
}

fn make_key(revision: Revision, key: &Vec<u8>) -> Vec<u8> {
	let mut key = key.clone();
	let mut revvec = (Revision::max_value() - revision).to_le_bytes()[..].to_vec();
	key.append(&mut revvec);
	key
}

impl RocksRevDB {
	pub fn new(
		db: DB,
		data_cf: String,
		journal_cf: String,
	) -> Result<Self, RocksRevDBError> {
		let mut this = Self {
			db,
			data_cf,
			journal_cf,
			revision: 0,
		};

		this.revision = this.fetch_revision()?;
		Ok(this)
	}

	fn fetch_revision(&self) -> Result<Revision, RocksRevDBError> {
		let journal_cf = self.db.cf_handle(&self.journal_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		let revraw = self.db.get_cf(journal_cf, b"revision")?;

		if let Some(revraw) = revraw {
			if revraw.len() == 8 {
				let mut revarr = [0u8; 8];
				revarr.copy_from_slice(&revraw);
				return Ok(u64::from_le_bytes(revarr))
			}
		} else {
			return Ok(0)
		}

		Err(RocksRevDBError::InvalidRevisionData)
	}

	fn commit_revision(&self, revision: Revision) -> Result<(), RocksRevDBError> {
		let revarr = revision.to_le_bytes();
		let journal_cf = self.db.cf_handle(&self.journal_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		self.db.put_cf(journal_cf, b"revision", &revarr)?;

		Ok(())
	}

	fn fetch_journal(&self, revision: Revision) -> Result<Vec<Vec<u8>>, RocksRevDBError> {
		let journal_cf = self.db.cf_handle(&self.journal_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		let journaldata = self.db.get_cf(journal_cf, &(b"journal", revision).encode()[..])?;

		match journaldata {
			Some(journaldata) => {
				if let Ok(keys) = Vec::<Vec<u8>>::decode(&mut &journaldata[..]) {
					Ok(keys)
				} else {
					Err(RocksRevDBError::InvalidJournalData)
				}
			},
			None => {
				Ok(Vec::new())
			},
		}
	}

	fn remove_journal(&self, revision: Revision) -> Result<(), RocksRevDBError> {
		let journal_cf = self.db.cf_handle(&self.journal_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		self.db.delete_cf(&journal_cf, &(b"journal", revision).encode()[..])?;

		Ok(())
	}

	fn commit_journal(&self, revision: Revision, keys: Vec<Vec<u8>>) -> Result<(), RocksRevDBError> {
		let journal_cf = self.db.cf_handle(&self.journal_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		self.db.put_cf(journal_cf, &(b"journal", revision).encode()[..], &keys.encode())?;

		Ok(())
	}

	fn remove_key(&self, revision: Revision, key: &Vec<u8>) -> Result<(), RocksRevDBError> {
		let data_cf = self.db.cf_handle(&self.data_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		self.db.delete_cf(&data_cf, make_key(revision, key))?;

		Ok(())
	}

	fn fetch_key(&self, revision: Revision, key: &Vec<u8>) -> Result<Option<Vec<u8>>, RocksRevDBError> {
		let data_cf = self.db.cf_handle(&self.data_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		let mut iter = self.db.prefix_iterator_cf(data_cf, make_key(revision, key));
		let value = iter.next();

		match value {
			None => Ok(None),
			Some((_, value)) => {
				if let Ok(value) = Option::<Vec<u8>>::decode(&mut &value[..]) {
					Ok(value)
				} else {
					Err(RocksRevDBError::InvalidData)
				}
			},
		}
	}

	fn commit_key(&self, revision: Revision, key: Vec<u8>, value: Option<Vec<u8>>) -> Result<(), RocksRevDBError> {
		let data_cf = self.db.cf_handle(&self.data_cf)
			.ok_or(RocksRevDBError::InvalidColumnFamily)?.clone();
		self.db.put_cf(data_cf, make_key(revision, &key), &value.encode())?;

		Ok(())
	}
}

impl RevDB for RocksRevDB {
	type Key = Vec<u8>;
	type Value = Option<Vec<u8>>;
	type Error = RocksRevDBError;

	fn revision(&self) -> Revision {
		self.revision
	}

	fn revert_to(&mut self, target: u64) -> Result<(), Self::Error> {
		if target > self.revision {
			return Err(RocksRevDBError::InvalidRevertTarget)
		}

		let mut current = self.revision;
		while current > target {
			let keys = self.fetch_journal(current)?;
			for key in keys {
				self.remove_key(current, &key)?;
			}
			self.remove_journal(current)?;
			current -= 1;
		}

		self.commit_revision(target)?;
		self.revision = target;

		Ok(())
	}

	fn get(&self, target: Revision, key: &Self::Key) -> Result<Self::Value, Self::Error> {
		if target > self.revision {
			return Err(RocksRevDBError::NoRevision)
		}

		self.fetch_key(target, key)
	}

	fn commit(
		&mut self,
		values: impl IntoIterator<Item=(Self::Key, Self::Value)>
	) -> Result<Revision, Self::Error> {
		let new = self.revision + 1;

		let mut keys = Vec::new();
		for (key, value) in values {
			self.commit_key(new, key.clone(), value)?;
			keys.push(key);
		}

		self.commit_journal(new, keys)?;
		self.commit_revision(new)?;
		self.revision = new;

		Ok(new)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn should_handle_commit_revert() {
		let dbdir = tempfile::tempdir().unwrap();
		let mut ropts = rocksdb::Options::default();
		ropts.create_if_missing(true);
		ropts.create_missing_column_families(true);
		let rdb = rocksdb::DB::open_cf(
			&ropts,
			dbdir.path().join("testdb"), &["data", "journal"]
		).unwrap();
		let mut db = RocksRevDB::new(rdb, "data".into(), "journal".into()).unwrap();

		assert_eq!(db.revision(), 0);
		assert_eq!(db.get(0, &vec![1]), Ok(None));

		db.commit(vec![(vec![1], Some(vec![5]))]).unwrap();
		db.commit(vec![(vec![1], Some(vec![7]))]).unwrap();
		db.commit(vec![(vec![1], None)]).unwrap();
		db.commit(vec![(vec![1], Some(vec![9]))]).unwrap();

		assert_eq!(db.revision(), 4);
		assert_eq!(db.get(1, &vec![1]), Ok(Some(vec![5])));
		assert_eq!(db.get(2, &vec![1]), Ok(Some(vec![7])));
		assert_eq!(db.get(3, &vec![1]), Ok(None));
		assert_eq!(db.get(4, &vec![1]), Ok(Some(vec![9])));

		db.revert_to(2).unwrap();
		assert_eq!(db.get(1, &vec![1]), Ok(Some(vec![5])));
		assert_eq!(db.get(2, &vec![1]), Ok(Some(vec![7])));
		assert_eq!(db.get(3, &vec![1]), Err(RocksRevDBError::NoRevision));
		assert_eq!(db.get(4, &vec![1]), Err(RocksRevDBError::NoRevision));
	}
}
