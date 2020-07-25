use crate::{RootStatus, Construct, Backend, ReadBackend, WriteBackend, Sequence, Raw, Dangling, Error, Index, Leak, Tree, Owned};

const LEN_INDEX: Index = Index::root().right();
const ITEM_ROOT_INDEX: Index = Index::root().left();

/// A tree with length mixed in.
pub struct LengthMixed<R: RootStatus, C: Construct, S: Sequence<Construct=C, RootStatus=Dangling>> {
	raw: Raw<R, C>,
	inner: S,
}

impl<R: RootStatus, C: Construct, S> LengthMixed<R, C, S> where
	S: Sequence<Construct=C, RootStatus=Dangling>,
	C::Value: From<usize> + Into<usize>,
{
	/// Reconstruct the mixed-length tree.
	pub fn reconstruct<DB: WriteBackend<Construct=C> + ?Sized, F>(
		root: C::Value,
		db: &mut DB,
		f: F
	) -> Result<Self, Error<DB::Error>> where
		F: FnOnce(Raw<Dangling, C>, &mut DB, usize) -> Result<S, Error<DB::Error>>,
	{
		let raw = Raw::<R, C>::from_leaked(root);
		let len: usize = raw.get(db, LEN_INDEX)?
			.ok_or(Error::CorruptedDatabase)?
			.into();
		let inner_raw = raw.subtree(db, ITEM_ROOT_INDEX)?;

		let inner = f(inner_raw, db, len)?;
		Ok(Self { inner, raw })
	}

	/// Deconstruct the mixed-length tree.
	pub fn deconstruct<DB: ReadBackend<Construct=C> + ?Sized>(
		self,
		db: &mut DB
	) -> Result<C::Value, Error<DB::Error>> {
		self.raw.get(db, LEN_INDEX)?;
		self.raw.get(db, ITEM_ROOT_INDEX)?;
		Ok(self.raw.root())
	}

	/// Call with the inner sequence.
	pub fn with<DB: Backend<Construct=C> + ?Sized, RT, F>(
		&self,
		db: &mut DB,
		f: F
	) -> Result<RT, Error<DB::Error>> where
		F: FnOnce(&S, &mut DB) -> Result<RT, Error<DB::Error>>
	{
		f(&self.inner, db)
	}

	/// Call with a mutable reference to the inner sequence.
	pub fn with_mut<DB: WriteBackend<Construct=C> + ?Sized, RT, F>(
		&mut self,
		db: &mut DB,
		f: F
	) -> Result<RT, Error<DB::Error>> where
		F: FnOnce(&mut S, &mut DB) -> Result<RT, Error<DB::Error>>
	{
		let ret = f(&mut self.inner, db)?;
		let new_len = self.inner.len();
		let new_inner_root = self.inner.root();

		self.raw.set(db, ITEM_ROOT_INDEX, new_inner_root)?;
		self.raw.set(db, LEN_INDEX, new_len.into())?;

		Ok(ret)
	}
}

impl<C: Construct, S> LengthMixed<Owned, C, S> where
	S: Sequence<Construct=C, RootStatus=Dangling> + Leak,
	C::Value: From<usize> + Into<usize>,
{
	/// Create a new mixed-length tree.
	pub fn create<DB: WriteBackend<Construct=C> + ?Sized, OS, F>(
		db: &mut DB,
		f: F
	) -> Result<Self, Error<DB::Error>> where
		F: FnOnce(&mut DB) -> Result<OS, Error<DB::Error>>,
		OS: Sequence<Construct=C> + Leak<Metadata=S::Metadata>,
	{
		let inner = f(db)?;
		let len = inner.len();
		let mut raw = Raw::default();

		raw.set(db, ITEM_ROOT_INDEX, inner.root())?;
		raw.set(db, LEN_INDEX, len.into())?;
		let metadata = inner.metadata();
		inner.drop(db)?;
		let dangling_inner = S::from_leaked(metadata);

		Ok(Self { raw, inner: dangling_inner })
	}
}

impl<R: RootStatus, C: Construct, S> Tree for LengthMixed<R, C, S> where
	S: Sequence<Construct=C, RootStatus=Dangling>,
{
	type RootStatus = R;
	type Construct = C;

	fn root(&self) -> C::Value {
		self.raw.root()
	}

	fn drop<DB: WriteBackend<Construct=C> + ?Sized>(
		self,
		db: &mut DB
	) -> Result<(), Error<DB::Error>> {
		self.inner.drop(db)?;
		self.raw.drop(db)?;
		Ok(())
	}

	fn into_raw(self) -> Raw<R, C> {
		self.raw
	}
}

impl<R: RootStatus, C: Construct, S> Sequence for LengthMixed<R, C, S> where
	S: Sequence<Construct=C, RootStatus=Dangling>,
{
	fn len(&self) -> usize {
		self.inner.len()
	}
}

impl<R: RootStatus, C: Construct, S> Leak for LengthMixed<R, C, S> where
	S: Sequence<Construct=C, RootStatus=Dangling> + Leak,
{
	type Metadata = (C::Value, S::Metadata);

	fn metadata(&self) -> Self::Metadata {
		let inner_metadata = self.inner.metadata();
		let raw_metadata = self.raw.metadata();

		(raw_metadata, inner_metadata)
	}

	fn from_leaked((raw_metadata, inner_metadata): Self::Metadata) -> Self {
		Self {
			raw: Raw::from_leaked(raw_metadata),
			inner: S::from_leaked(inner_metadata),
		}
	}
}
