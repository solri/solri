use bm::{ReadBackend, WriteBackend, Construct, Error, DanglingVector, Leak};
use bm::utils::vector_tree;
use primitive_types::{H256, H512};
use generic_array::{GenericArray, ArrayLength};
use vecarray::VecArray;
use typenum::Unsigned;
use core::convert::TryFrom;
use alloc::vec::Vec;
use crate::{ElementalFixedVecRef, ElementalFixedVec, IntoCompositeVectorTree,
            IntoCompactVectorTree, IntoTree, FromTree, FromCompositeVectorTree,
            FromCompactVectorTree, Compact, CompactRef, CompatibleConstruct};

impl<'a, T, L: ArrayLength<T>> IntoTree for CompactRef<'a, GenericArray<T, L>> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoCompactVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>> IntoTree for Compact<GenericArray<T, L>> where
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompactVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>> FromTree for Compact<GenericArray<T, L>> where
    T: Default,
    ElementalFixedVec<T>: FromCompactVectorTree,
{
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<T>::from_compact_vector_tree(root, db, L::to_usize(), None)?;
        let mut ret = GenericArray::default();
        for (i, v) in value.0.into_iter().enumerate() {
            ret[i] = v;
        }
        Ok(Self(ret))
    }
}

impl<'a, T, L: Unsigned> IntoTree for CompactRef<'a, VecArray<T, L>> where
    for<'b> ElementalFixedVecRef<'b, T>: IntoCompactVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: Unsigned> IntoTree for Compact<VecArray<T, L>> where
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompactVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0).into_compact_vector_tree(db, None)
    }
}

impl<T, L: Unsigned> FromTree for Compact<VecArray<T, L>> where
    T: Default,
    ElementalFixedVec<T>: FromCompactVectorTree,
{
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<T>::from_compact_vector_tree(root, db, L::to_usize(), None)?;
        Ok(Self(VecArray::try_from(value.0).map_err(|_| Error::CorruptedDatabase)?))
    }
}

impl IntoTree for H256 {
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0.as_ref()).into_compact_vector_tree(db, None)
    }
}

impl FromTree for H256 {
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<u8>::from_compact_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

impl IntoTree for H512 {
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self.0.as_ref()).into_compact_vector_tree(db, None)
    }
}

impl FromTree for H512 {
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<u8>::from_compact_vector_tree(root, db, 32, None)?;
        Ok(Self::from_slice(value.0.as_ref()))
    }
}

macro_rules! impl_fixed_array {
    ( $( $n:expr ),* ) => { $(
        impl<T> IntoTree for [T; $n] where
            for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree,
        {
            fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
            }
        }

        impl<T> FromTree for [T; $n] where
            T: Default + Copy,
            for<'a> ElementalFixedVec<T>: FromCompositeVectorTree,
        {
            fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, $n, None)?;
                let mut ret = [T::default(); $n];
                for (i, v) in value.0.into_iter().enumerate() {
                    ret[i] = v;
                }
                Ok(ret)
            }
        }
    )* }
}

impl_fixed_array!(1, 2, 3, 4, 5, 6, 7, 8,
                  9, 10, 11, 12, 13, 14, 15, 16,
                  17, 18, 19, 20, 21, 22, 23, 24,
                  25, 26, 27, 28, 29, 30, 31, 32);

impl<T, L: ArrayLength<T>> IntoTree for GenericArray<T, L> where
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
    }
}

impl<T, L: ArrayLength<T>> FromTree for GenericArray<T, L> where
    for<'a> ElementalFixedVec<T>: FromCompositeVectorTree,
{
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, L::to_usize(), None)?;
        Ok(GenericArray::from_exact_iter(value.0)
           .expect("Fixed vec must build vector with L::as_usize; qed"))
    }
}

impl<T, L: Unsigned> IntoTree for VecArray<T, L> where
    for<'a> ElementalFixedVecRef<'a, T>: IntoCompositeVectorTree,
{
    fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        ElementalFixedVecRef(&self[..]).into_composite_vector_tree(db, None)
    }
}

impl<T, L: Unsigned> FromTree for VecArray<T, L> where
    for<'a> ElementalFixedVec<T>: FromCompositeVectorTree,
{
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        let value = ElementalFixedVec::<T>::from_composite_vector_tree(root, db, L::to_usize(), None)?;
        Ok(VecArray::try_from(value.0).map_err(|_| Error::CorruptedDatabase)?)
    }
}

impl FromTree for () {
    fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, _db: &mut DB) -> Result<Self, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        if root == &Default::default() {
            Ok(())
        } else {
            Err(Error::CorruptedDatabase)
        }
    }
}

impl IntoTree for () {
    fn into_tree<DB: WriteBackend>(&self, _db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
        DB::Construct: CompatibleConstruct,
    {
        Ok(Default::default())
    }
}

macro_rules! impl_tuple {
    ($len:expr, $($i:ident => $t:ident),+) => {
        impl<$($t: FromTree),+> FromTree for ($($t,)+) {
            fn from_tree<DB: ReadBackend>(root: &<DB::Construct as Construct>::Value, db: &mut DB) -> Result<Self, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                let vector = DanglingVector::<DB::Construct>::from_leaked(
                    (root.clone(), $len, None)
                );
                let mut i = 0;
                Ok(($({
                    let value = <$t>::from_tree(&vector.get(db, i)?, db)?;
                    #[allow(unused_assignments)] {
                        i += 1;
                    }
                    value
                }),+))
            }
        }

        impl<$($t: IntoTree),+> IntoTree for ($($t),+) {
            fn into_tree<DB: WriteBackend>(&self, db: &mut DB) -> Result<<DB::Construct as Construct>::Value, Error<DB::Error>> where
                DB::Construct: CompatibleConstruct,
            {
                let ($($i),+) = self;
                let mut vector = Vec::new();
                $({
                    vector.push($i.into_tree(db)?);
                })+
                vector_tree(&vector, db, None)
            }
        }
    }
}

impl_tuple!(2, a => A, b => B);
impl_tuple!(3, a => A, b => B, c => C);
impl_tuple!(4, a => A, b => B, c => C, d => D);
impl_tuple!(5, a => A, b => B, c => C, d => D, e => E);
impl_tuple!(6, a => A, b => B, c => C, d => D, e => E, f => F);
impl_tuple!(7, a => A, b => B, c => C, d => D, e => E, f => F, g => G);
impl_tuple!(8, a => A, b => B, c => C, d => D, e => E, f => F, g => G, h => H);
impl_tuple!(9, a => A, b => B, c => C, d => D, e => E, f => F, g => G, h => H, i => I);
