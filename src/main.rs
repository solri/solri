use primitive_types::{H256, H512};
use bm::Leak;
use bm_le::{FromTree, IntoTree, tree_root};
use blockchain::traits::{Block as BlockT};
use sha3::Sha3_256;

pub type AccountId = u64;
pub type Difficulty = u128;
pub type Balance = u128;
pub type Nonce = u64;
pub type Public = H256;
pub type Signature = H512;
pub type Hash = Sha3_256;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum TransferId {
    Existing(AccountId),
    New(Public),
}

impl<DB> FromTree<DB> for TransferId where
    DB: bm_le::Backend<Intermediate=bm_le::Intermediate, End=bm_le::End>
{
    fn from_tree(
        root: &bm_le::ValueOf<DB>,
        db: &DB
    ) -> Result<Self, bm_le::Error<DB::Error>> {
        match root {
            bm_le::Value::End(_) => {
                Ok(TransferId::Existing(u64::from_tree(root, db)?))
            },
            bm_le::Value::Intermediate(_) => {
                let raw = bm::DanglingRaw::from_leaked(root.clone());
                Ok(TransferId::New(
                    H256::from_tree(&raw.get(db, bm::Index::root().left())?.ok_or(
                        bm_le::Error::CorruptedDatabase
                    )?, db)?
                ))
            },
        }
    }
}

impl<DB> IntoTree<DB> for TransferId where
    DB: bm_le::Backend<Intermediate=bm_le::Intermediate, End=bm_le::End>
{
    fn into_tree(
        &self,
        db: &mut DB
    ) -> Result<bm_le::ValueOf<DB>, bm_le::Error<DB::Error>> {
        match self {
            TransferId::Existing(existing) => {
                existing.into_tree(db)
            },
            TransferId::New(new) => {
                let mut raw = bm::DanglingRaw::default();
                let left = new.into_tree(db)?;
                raw.set(db, bm::Index::root().left(), left)?;
                Ok(raw.metadata())
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree)]
pub struct UnsealedBlock {
    pub parent_id: H256,
    pub state_root: H256,
    pub coinbase: TransferId,
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree)]
pub struct Block {
    pub unsealed: UnsealedBlock,
    pub proof: H256,
}

impl BlockT for Block {
    type Identifier = H256;

    fn parent_id(&self) -> Option<H256> {
        if self.unsealed.parent_id == H256::default() {
            None
        } else {
            Some(self.unsealed.parent_id.clone())
        }
    }

    fn id(&self) -> H256 {
        tree_root::<Hash, _>(self)
    }
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree)]
pub struct Account {
    pub balance: Balance,
    pub nonce: Nonce,
    pub public: Public,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree)]
pub struct UnsealedTransaction {
    pub from: AccountId,
    pub to: TransferId,
    pub amount: Balance,
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree)]
pub struct Transaction {
    pub unsealed: UnsealedTransaction,
    pub signature: Signature,
}

fn main() {
    println!("Hello, world!");
}
