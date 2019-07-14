use primitive_types::{H256, H512};
use bm_le::{FromTree, IntoTree, tree_root};
use blockchain::traits::{Block as BlockT, BlockExecutor, SimpleBuilderExecutor, AsExternalities};
use parity_codec::{Encode, Decode};
use sha3::Sha3_256;

pub type AccountId = u64;
pub type Difficulty = u128;
pub type Balance = u128;
pub type Nonce = u64;
pub type Public = H256;
pub type Signature = H512;
pub type Hash = Sha3_256;

const DIFFICULTY: Difficulty = 1;
const REWARD: Balance = 5;

fn is_all_zero(arr: &[u8]) -> bool {
    arr.iter().all(|i| *i == 0)
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub enum TransferId {
    Existing(AccountId),
    New(Public),
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct UnsealedBlock {
    pub parent_id: Option<H256>,
    pub state_root: H256,
    pub coinbase: Option<TransferId>,
    pub transactions: Vec<Transaction>,
}

impl UnsealedBlock {
    pub fn seal(self) -> Block {
        let mut block = Block {
            unsealed: self,
            proof: 0,
        };

        while !block.is_valid() {
            block.proof += 1;
        }

        block
    }
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct Block {
    pub unsealed: UnsealedBlock,
    pub proof: u64,
}

impl Block {
    pub fn is_valid(&self) -> bool {
        is_all_zero(&self.id()[0..DIFFICULTY as usize])
    }

    pub fn genesis() -> Self {
        let state = Vec::<Account>::new();
        let state_root = tree_root::<Hash, _>(&state);

        let unsealed = UnsealedBlock {
            parent_id: None,
            state_root,
            coinbase: None,
            transactions: Vec::new(),
        };

        unsealed.seal()
    }
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

impl UnsealedTransaction {
    pub fn seal(self, keypair: &schnorrkel::Keypair) -> Transaction {
        let context = schnorrkel::signing_context(b"Solri Transaction");

        let message = tree_root::<Hash, _>(&self);
        let sig = keypair.sign(context.bytes(&message[..]));

        let signature = H512::from(sig.to_bytes());

        Transaction {
            unsealed: self,
            signature,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, FromTree, IntoTree, Encode, Decode)]
pub struct Transaction {
    pub unsealed: UnsealedTransaction,
    pub signature: Signature,
}

impl Transaction {
    pub fn verify(&self, state: &[Account]) -> Result<(), Error> {
        let context = schnorrkel::signing_context(b"Solri Transaction");
        let from = self.unsealed.from as usize;

        if state.len() <= from {
            return Err(Error::AccountIdOutOfRange)
        }

        let pubkey = match schnorrkel::PublicKey::from_bytes(
            &state[from].public[..]
        ) {
            Ok(pubkey) => pubkey,
            Err(_) => return Err(Error::InvalidSignature),
        };
        let sig = match schnorrkel::Signature::from_bytes(
            &self.signature[..]
        ) {
            Ok(sig) => sig,
            Err(_) => return Err(Error::InvalidSignature),
        };
        let message = tree_root::<Hash, _>(&self.unsealed);

        pubkey.verify(context.bytes(&message[..]), &sig)
            .map_err(|_| Error::InvalidSignature)
    }
}

#[derive(Debug)]
pub enum Error {
    Backend(Box<std::error::Error>),
    DifficultyTooLow,
    InvalidSignature,
    AccountIdOutOfRange,
    AccountNotEnoughBalance,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error { }

impl From<Error> for blockchain::import::Error {
    fn from(error: Error) -> Self {
	blockchain::import::Error::Executor(Box::new(error))
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, FromTree, IntoTree, Encode, Decode)]
pub struct State(Vec<Account>);

pub trait StateExternalities: AsRef<Vec<Account>> + AsMut<Vec<Account>> {
    fn tree_root(&self) -> H256;
    fn transfer(
        &mut self,
        from: Option<AccountId>,
        to: TransferId,
        amount: Balance,
    ) -> Result<(), Error> {
        match from {
            Some(from) => {
                if self.as_ref().len() <= from as usize {
                    return Err(Error::AccountIdOutOfRange)
                }

                if self.as_ref()[from as usize].balance < amount {
                    return Err(Error::AccountNotEnoughBalance)
                }
            },
            None => (),
        }

        match to {
            TransferId::Existing(to) => {
                if self.as_ref().len() <= to as usize {
                    return Err(Error::AccountIdOutOfRange)
                }
            },
            TransferId::New(_) => (),
        }

        match from {
            Some(from) => {
                self.as_mut()[from as usize].nonce += 1;
            },
            None => (),
        }

        match to {
            TransferId::Existing(to) => {
                self.as_mut()[to as usize].balance += amount;
            },
            TransferId::New(public) => {
                self.as_mut().push(Account {
                    public,
                    nonce: 0,
                    balance: amount,
                });
            },
        }

        Ok(())
    }
}

impl AsRef<Vec<Account>> for State {
    fn as_ref(&self) -> &Vec<Account> {
        &self.0
    }
}

impl AsMut<Vec<Account>> for State {
    fn as_mut(&mut self) -> &mut Vec<Account> {
        &mut self.0
    }
}

impl StateExternalities for State {
    fn tree_root(&self) -> H256 {
        tree_root::<Hash, _>(self)
    }
}

impl AsExternalities<dyn StateExternalities> for State {
    fn as_externalities(&mut self) -> &mut (dyn StateExternalities + 'static) {
        self
    }
}

#[derive(Clone)]
pub struct Executor;

impl BlockExecutor for Executor {
    type Error = Error;
    type Block = Block;
    type Externalities = dyn StateExternalities + 'static;

    fn execute_block(
	&self,
	block: &Self::Block,
	state: &mut Self::Externalities,
    ) -> Result<(), Error> {
        if !block.is_valid() {
            return Err(Error::DifficultyTooLow);
        }

        for transaction in &block.unsealed.transactions {
            transaction.verify(state.as_ref())?;
            state.transfer(
                Some(transaction.unsealed.from),
                transaction.unsealed.to.clone(),
                transaction.unsealed.amount
            )?;
        }

        if let Some(coinbase) = block.unsealed.coinbase.clone() {
            state.transfer(
                None,
                coinbase,
                REWARD
            )?;
        }

        Ok(())
    }
}

impl SimpleBuilderExecutor for Executor {
    type BuildBlock = UnsealedBlock;
    type Extrinsic = Transaction;
    type Inherent = ();

    fn initialize_block(
	&self,
	block: &Self::Block,
	_state: &mut Self::Externalities,
	_inherent: (),
    ) -> Result<Self::BuildBlock, Self::Error> {
        Ok(UnsealedBlock {
            parent_id: Some(block.id()),
            state_root: block.unsealed.state_root,
            transactions: Vec::new(),
            coinbase: None,
        })
    }

    fn apply_extrinsic(
	&self,
	block: &mut Self::BuildBlock,
	extrinsic: Self::Extrinsic,
	state: &mut Self::Externalities,
    ) -> Result<(), Self::Error> {
        extrinsic.verify(state.as_ref())?;
        state.transfer(
            Some(extrinsic.unsealed.from),
            extrinsic.unsealed.to.clone(),
            extrinsic.unsealed.amount
        )?;
        block.transactions.push(extrinsic);

        block.state_root = state.tree_root();
        Ok(())
    }

    fn finalize_block(
	&self,
	block: &mut Self::BuildBlock,
	state: &mut Self::Externalities,
    ) -> Result<(), Self::Error> {
        if let Some(coinbase) = block.coinbase.clone() {
            state.transfer(
                None,
                coinbase,
                REWARD
            )?;
        }

        block.state_root = state.tree_root();
        Ok(())
    }
}
