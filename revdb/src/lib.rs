/// Revision database.
pub trait RevDB {
	/// Key type.
    type Key;
	/// Value type.
    type Value;
	/// Error type.
    type Error;

    /// Current revision of the database.
    fn revision(&self) -> u64;
    /// Revert the database back to an earlier revision.
    fn revert_to(&mut self, revision: u64) -> Result<(), Self::Error>;
    /// Get value from the database, of a revision.
    fn get(&self, revision: u64, key: &[u8]) -> Result<Vec<u8>, Self::Error>;
    /// Commit values into the database to form a new revision,
    /// returns the new revision number.
    fn commit<'a, 'b>(
        &mut self,
        values: impl Iterator<Item=(&'a [u8], &'b [u8])>
    ) -> Result<u64, Self::Error>;
}
