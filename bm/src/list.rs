use crate::traits::{ReadBackend, WriteBackend, Construct, RootStatus, Dangling, Owned, Leak, Error, Tree, Sequence};
use crate::vector::Vector;
use crate::raw::Raw;
use crate::length::LengthMixed;

/// `List` with owned root.
pub type OwnedList<C> = List<Owned, C>;

/// `List` with dangling root.
pub type DanglingList<C> = List<Dangling, C>;

/// Binary merkle vector.
pub struct List<R: RootStatus, C: Construct>(LengthMixed<R, C, Vector<Dangling, C>>);

impl<R: RootStatus, C: Construct> List<R, C> where
    C::Value: From<usize> + Into<usize>,
{
    /// Get value at index.
    pub fn get<DB: ReadBackend<Construct=C> + ?Sized>(&self, db: &mut DB, index: usize) -> Result<C::Value, Error<DB::Error>> {
        self.0.with(db, |tuple, db| tuple.get(db, index))
    }

    /// Set value at index.
    pub fn set<DB: WriteBackend<Construct=C> + ?Sized>(&mut self, db: &mut DB, index: usize, value: C::Value) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.set(db, index, value))
    }

    /// Push a new value to the vector.
    pub fn push<DB: WriteBackend<Construct=C> + ?Sized>(&mut self, db: &mut DB, value: C::Value) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.push(db, value))
    }

    /// Pop a value from the vector.
    pub fn pop<DB: WriteBackend<Construct=C> + ?Sized>(&mut self, db: &mut DB) -> Result<Option<C::Value>, Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.pop(db))
    }

    /// Deconstruct the vector into one single hash value, and leak only the hash value.
    pub fn deconstruct<DB: ReadBackend<Construct=C> + ?Sized>(self, db: &mut DB) -> Result<C::Value, Error<DB::Error>> {
        self.0.deconstruct(db)
    }

    /// Reconstruct the vector from a single hash value.
    pub fn reconstruct<DB: WriteBackend<Construct=C> + ?Sized>(root: C::Value, db: &mut DB, max_len: Option<usize>) -> Result<Self, Error<DB::Error>> {
        Ok(Self(LengthMixed::reconstruct(root, db, |tuple_raw, _db, len| {
            Ok(Vector::<Dangling, C>::from_raw(tuple_raw, len, max_len))
        })?))
    }
}

impl<R: RootStatus, C: Construct> Tree for List<R, C> where
    C::Value: From<usize> + Into<usize>,
{
    type RootStatus = R;
    type Construct = C;

    fn root(&self) -> C::Value {
        self.0.root()
    }

    fn drop<DB: WriteBackend<Construct=C> + ?Sized>(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.0.drop(db)
    }

    fn into_raw(self) -> Raw<R, C> {
        self.0.into_raw()
    }
}

impl<R: RootStatus, C: Construct> Sequence for List<R, C> where
    C::Value: From<usize> + Into<usize>,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<R: RootStatus, C: Construct> Leak for List<R, C> where
    C::Value: From<usize> + Into<usize>,
{
    type Metadata = <LengthMixed<R, C, Vector<Dangling, C>> as Leak>::Metadata;

    fn metadata(&self) -> Self::Metadata {
        self.0.metadata()
    }

    fn from_leaked(metadata: Self::Metadata) -> Self {
        Self(LengthMixed::from_leaked(metadata))
    }
}

impl<C: Construct> List<Owned, C> where
    C::Value: From<usize> + Into<usize>
{
    /// Create a new vector.
    pub fn create<DB: WriteBackend<Construct=C> + ?Sized>(
        db: &mut DB,
        max_len: Option<usize>
    ) -> Result<Self, Error<DB::Error>> {
        Ok(Self(LengthMixed::create(db, |db| Vector::<Owned, _>::create(db, 0, max_len))?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use generic_array::GenericArray;
    use sha2::Sha256;

    type InheritedInMemory = crate::memory::InMemoryBackend<crate::InheritedDigestConstruct<Sha256, ListValue>>;
    type UnitInMemory = crate::memory::InMemoryBackend<crate::UnitDigestConstruct<Sha256, ListValue>>;

    #[derive(Clone, PartialEq, Eq, Debug, Default, Ord, PartialOrd, Hash)]
    struct ListValue(Vec<u8>);

    impl From<GenericArray<u8, typenum::U32>> for ListValue {
        fn from(array: GenericArray<u8, typenum::U32>) -> ListValue {
            ListValue(array.as_slice().to_vec())
        }
    }

    impl AsRef<[u8]> for ListValue {
        fn as_ref(&self) -> &[u8] {
            self.0.as_ref()
        }
    }

    impl From<usize> for ListValue {
        fn from(value: usize) -> Self {
            ListValue((&(value as u64).to_le_bytes()[..]).into())
        }
    }

    impl Into<usize> for ListValue {
        fn into(self) -> usize {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&self.0[0..8]);
            u64::from_le_bytes(raw) as usize
        }
    }

    #[test]
    fn test_push_pop_inherited() {
        let mut db = InheritedInMemory::default();

        let mut vec = List::create(&mut db, None).unwrap();
        let mut roots = Vec::new();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into()).unwrap();
            roots.push(vec.root());
        }
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            assert_eq!(vec.root(), roots.pop().unwrap());
            let value = vec.pop(&mut db).unwrap();
            assert_eq!(value, Some(i.into()));
            assert_eq!(vec.len(), i);
        }
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_push_pop_unit() {
        let mut db = UnitInMemory::default();

        let mut vec = List::create(&mut db, None).unwrap();
        let mut roots = Vec::new();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into()).unwrap();
            roots.push(vec.root());
        }
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            assert_eq!(vec.root(), roots.pop().unwrap());
            let value = vec.pop(&mut db).unwrap();
            assert_eq!(value, Some(i.into()));
            assert_eq!(vec.len(), i);
        }
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_set() {
        let mut db = InheritedInMemory::default();
        let mut vec = OwnedList::create(&mut db, None).unwrap();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, Default::default()).unwrap();
        }

        for i in 0..100 {
            vec.set(&mut db, i, i.into()).unwrap();
        }
        for i in 0..100 {
            assert_eq!(vec.get(&mut db, i).unwrap(), i.into());
        }
    }

    #[test]
    fn test_deconstruct_reconstruct() {
        let mut db = InheritedInMemory::default();
        let mut vec = OwnedList::create(&mut db, None).unwrap();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into()).unwrap();
        }
        let vec_hash = vec.deconstruct(&mut db).unwrap();

        let vec = OwnedList::reconstruct(vec_hash, &mut db, None).unwrap();
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            assert_eq!(vec.get(&mut db, i).unwrap(), i.into());
        }
    }
}
