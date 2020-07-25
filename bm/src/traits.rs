/// Construct for a merkle tree.
pub trait Construct: Sized {
    /// Value stored in this merkle database.
    type Value: Clone + Default;

    /// Get the intermediate value of given left and right child.
    fn intermediate_of(left: &Self::Value, right: &Self::Value) -> Self::Value;
    /// Get or create the empty value given a backend. `empty_at(0)`
    /// should always equal to `Value::End(Default::default())`.
    fn empty_at<DB: WriteBackend<Construct=Self> + ?Sized>(
        db: &mut DB,
        depth_to_bottom: usize
    ) -> Result<Self::Value, DB::Error>;
}

/// Represents a basic merkle tree with a known root.
pub trait Tree {
    /// Root status of the tree.
    type RootStatus: RootStatus;
    /// Construct of the tree.
    type Construct: Construct;

    /// Root of the merkle tree.
    fn root(&self) -> <Self::Construct as Construct>::Value;
    /// Drop the merkle tree.
    fn drop<DB: WriteBackend<Construct=Self::Construct> + ?Sized>(
        self,
        db: &mut DB
    ) -> Result<(), Error<DB::Error>>;
    /// Convert the tree into a raw tree.
    fn into_raw(self) -> crate::Raw<Self::RootStatus, Self::Construct>;
}

/// A merkle tree that is similar to a vector.
pub trait Sequence: Tree {
    /// The length of the tree.
    fn len(&self) -> usize;
}

/// Root status of a merkle tree.
pub trait RootStatus {
    /// Whether it is a dangling root.
    fn is_dangling() -> bool;
    /// Whether it is an owned root.
    fn is_owned() -> bool { !Self::is_dangling() }
}

/// Dangling root status.
pub struct Dangling;

impl RootStatus for Dangling {
    fn is_dangling() -> bool { true }
}

/// Owned root status.
pub struct Owned;

impl RootStatus for Owned {
    fn is_dangling() -> bool { false }
}

/// Set error.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error<DBError> {
    /// The database is corrupted.
    CorruptedDatabase,
    /// Value trying to access overflowed the list or vector.
    AccessOverflowed,
    /// Parameters are invalid.
    InvalidParameter,
    /// Backend database error.
    Backend(DBError),
}

impl<DBError> From<DBError> for Error<DBError> {
    fn from(err: DBError) -> Self {
        Error::Backend(err)
    }
}

/// Traits for a merkle database.
pub trait Backend {
    /// Construct of the backend.
    type Construct: Construct;
    /// Error type for DB access.
    type Error;
}

/// Read backend.
pub trait ReadBackend: Backend {
    /// Get an internal item by key.
    fn get(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<Option<(<Self::Construct as Construct>::Value, <Self::Construct as Construct>::Value)>, Self::Error>;
}

/// Write backend.
pub trait WriteBackend: ReadBackend {
    /// Rootify a key.
    fn rootify(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<(), Self::Error>;
    /// Unrootify a key.
    fn unrootify(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<(), Self::Error>;
    /// Insert a new internal item. None indicating that we do not
    /// know what the internal item is.
    fn insert(
        &mut self,
        key: <Self::Construct as Construct>::Value,
        value: (<Self::Construct as Construct>::Value, <Self::Construct as Construct>::Value)
    ) -> Result<(), Self::Error>;
}

/// Dynamic backend, where error is stripped.
#[derive(Default, Clone, Debug)]
pub struct DynBackend<Ba: Backend>(pub Ba);

impl<Ba: Backend> core::ops::Deref for DynBackend<Ba> {
	type Target = Ba;

	fn deref(&self) -> &Ba { &self.0 }
}

impl<Ba: Backend> core::ops::DerefMut for DynBackend<Ba> {
	fn deref_mut(&mut self) -> &mut Ba { &mut self.0 }
}

impl<Ba: Backend> Backend for DynBackend<Ba> {
	type Construct = Ba::Construct;
	type Error = ();
}

impl<Ba: ReadBackend> ReadBackend for DynBackend<Ba> {
	fn get(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<Option<(<Self::Construct as Construct>::Value, <Self::Construct as Construct>::Value)>, Self::Error> {
		self.0.get(key).map_err(|_| ())
	}
}

impl<Ba: WriteBackend> WriteBackend for DynBackend<Ba> {
	fn rootify(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<(), Self::Error> {
		self.0.rootify(key).map_err(|_| ())
	}

    fn unrootify(
        &mut self,
        key: &<Self::Construct as Construct>::Value,
    ) -> Result<(), Self::Error> {
		self.0.unrootify(key).map_err(|_| ())
	}

    fn insert(
        &mut self,
        key: <Self::Construct as Construct>::Value,
        value: (<Self::Construct as Construct>::Value, <Self::Construct as Construct>::Value)
    ) -> Result<(), Self::Error> {
		self.0.insert(key, value).map_err(|_| ())
	}
}

/// Leakable value, whose default behavior of drop is to leak.
pub trait Leak {
    /// Metadata to represent this merkle struct.
    type Metadata;

    /// Initialize from a previously leaked value.
    fn from_leaked(metadata: Self::Metadata) -> Self;
    /// Metadata of the value.
    fn metadata(&self) -> Self::Metadata;
}
