use bm_le::MaxVec;
use bm_le_derive::{FromTree, IntoTree};
use generic_array::{GenericArray, ArrayLength};

pub trait Config {
    type D: ArrayLength<u64>;
    type E: ArrayLength<u64>;
}

#[derive(IntoTree, FromTree)]
pub struct Container<C: Config> {
    a: u32,
    b: u64,
    c: u128,
    d: GenericArray<u64, C::D>,
    e: MaxVec<u64, C::E>,
}

#[derive(IntoTree, FromTree)]
pub enum EnumTest {
    A(u128),
    B {
        c: u64,
        d: u32,
    },
    E,
}
