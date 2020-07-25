extern crate alloc;

mod memory;

pub use crate::memory::{MemoryRevDB, MemoryRevDBError};

/// Revision type.
pub type Revision = u64;

/// Revision database.
pub trait RevDB {
	/// Key type.
    type Key;
	/// Value type.
    type Value;
	/// Error type.
    type Error;

    /// Current revision of the database.
    fn revision(&self) -> Revision;
    /// Revert the database back to an earlier revision.
    fn revert_to(&mut self, revision: u64) -> Result<(), Self::Error>;
    /// Get value from the database, of a revision.
    fn get(&self, revision: u64, key: &Self::Key) -> Result<Self::Value, Self::Error>;
    /// Commit values into the database to form a new revision,
    /// returns the new revision number.
    fn commit(
        &mut self,
        values: impl Iterator<Item=(Self::Key, Self::Value)>
    ) -> Result<Revision, Self::Error>;
}
