use primitive_types::{H256, H512};
use bm_le::{FromTree, IntoTree, tree_root};
use blockchain::traits::{Block as BlockT};
use parity_codec::{Encode, Decode};
use sha3::Sha3_256;

pub type AccountId = u64;
pub type Difficulty = u128;
pub type Balance = u128;
pub type Nonce = u64;
pub type Public = H256;
pub type Signature = H512;
pub type Hash = Sha3_256;

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub enum TransferId {
    Existing(AccountId),
    New(Public),
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct UnsealedBlock {
    pub parent_id: Option<H256>,
    pub state_root: H256,
    pub coinbase: TransferId,
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct Block {
    pub unsealed: UnsealedBlock,
    pub proof: H256,
}

impl BlockT for Block {
    type Identifier = H256;

    fn parent_id(&self) -> Option<H256> {
        self.unsealed.parent_id
    }

    fn id(&self) -> H256 {
        tree_root::<Hash, _>(self)
    }
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct Account {
    pub balance: Balance,
    pub nonce: Nonce,
    pub public: Public,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct UnsealedTransaction {
    pub from: AccountId,
    pub to: TransferId,
    pub amount: Balance,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct Transaction {
    pub unsealed: UnsealedTransaction,
    pub signature: Signature,
}

fn main() {
    println!("Hello, world!");
}
