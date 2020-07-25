use rocksdb::{DB, ColumnFamily};
use codec::{Encode, Decode};
use crate::{RevDB, Revision};

pub enum RocksRevDBError {
	/// Backend error.
	Backend(rocksdb::Error),
	/// Invalid revision metadata in database.
	InvalidRevisionData,
	/// Invalid journal data.
	InvalidJournalData,
	/// Invalid data.
	InvalidData,
	/// Revert target out of range.
	InvalidRevertTarget,
}

impl From<rocksdb::Error> for RocksRevDBError {
	fn from(err: rocksdb::Error) -> Self {
		Self::Backend(err)
	}
}

pub struct RocksRevDB {
	db: DB,
	data_cf: ColumnFamily,
	journal_cf: ColumnFamily,
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
		data_cf: ColumnFamily,
		journal_cf: ColumnFamily,
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
		let revraw = self.db.get_cf(&self.journal_cf, b"revision")?;

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
		self.db.put_cf(&self.journal_cf, b"revision", &revarr)?;

		Ok(())
	}

	fn fetch_journal(&self, revision: Revision) -> Result<Vec<Vec<u8>>, RocksRevDBError> {
		let journaldata = self.db.get_cf(&self.journal_cf, &(b"journal", revision).encode()[..])?;

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
		self.db.delete_cf(&self.journal_cf, &(b"journal", revision).encode()[..])?;

		Ok(())
	}

	fn commit_journal(&self, revision: Revision, keys: Vec<Vec<u8>>) -> Result<(), RocksRevDBError> {
		self.db.put_cf(&self.journal_cf, &(b"journal", revision).encode()[..], &keys.encode())?;

		Ok(())
	}

	fn remove_key(&self, revision: Revision, key: &Vec<u8>) -> Result<(), RocksRevDBError> {
		self.db.delete_cf(&self.data_cf, make_key(revision, key))?;

		Ok(())
	}

	fn fetch_key(&self, revision: Revision, key: &Vec<u8>) -> Result<Option<Vec<u8>>, RocksRevDBError> {
		let mut iter = self.db.prefix_iterator_cf(&self.data_cf, make_key(revision, key));
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
		self.db.put_cf(&self.data_cf, make_key(revision, &key), &value.encode())?;

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

		Ok(())
	}

    fn get(&self, target: Revision, key: &Self::Key) -> Result<Self::Value, Self::Error> {
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

		Ok(new)
	}
}
