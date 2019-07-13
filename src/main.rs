use primitive_types::{H256, H512};

pub type AccountId = u64;
pub type Difficulty = u128;
pub type Balance = u128;
pub type Nonce = u64;
pub type Public = H256;
pub type Signature = H512;

pub enum TransferId {
    Existing(AccountId),
    New(Public),
}

pub struct Block {
    pub unsealed: UnsealedBlock,
    pub proof: H256,
}

pub struct UnsealedBlock {
    pub parent_id: Option<H256>,
    pub state_root: H256,
    pub coinbase: TransferId,
    pub transactions: Vec<Transaction>,
}

pub struct Account {
    pub balance: Balance,
    pub nonce: Nonce,
    pub public: Public,
}

pub struct Transaction {
    pub unsealed: UnsealedTransaction,
    pub signature: Signature,
}

pub struct UnsealedTransaction {
    pub from: AccountId,
    pub to: TransferId,
    pub amount: Balance,
}

fn main() {
    println!("Hello, world!");
}
