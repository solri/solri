use primitive_types::H256;

pub type AccountId = u64;
pub type Difficulty = u128;
pub type Balance = u128;

pub struct Block {
    parent_id: Option<H256>,
    state_root: H256,
    coinbase: AccountId,
    difficulty: Difficulty,
    nonce: H256,
}

fn main() {
    println!("Hello, world!");
}
