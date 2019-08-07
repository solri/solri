use crate::{Error, Instance};
use std::sync::Arc;
use blockchain::{BlockExecutor, Block as BlockT};
#[cfg(feature = "parity-codec")]
use parity_codec::{Encode, Decode};

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode)]
pub struct Block {
	pub id: Vec<u8>,
	pub parent_id: Option<Vec<u8>>,
	pub difficulty: u64,
	pub timestamp: u64,
	pub data: Vec<u8>,
}

impl BlockT for Block {
    type Identifier = Vec<u8>;

    fn parent_id(&self) -> Option<Vec<u8>> {
		self.parent_id.clone()
    }

    fn id(&self) -> Vec<u8> {
		self.id.clone()
    }
}

pub trait CodeExternalities {
	fn code(&self) -> &Vec<u8>;
	fn code_mut(&mut self) -> &mut Vec<u8>;
}

#[derive(Default)]
pub struct Executor;

impl BlockExecutor for Executor {
    type Error = Error;
    type Block = Block;
    type Externalities = dyn CodeExternalities + 'static;

    fn execute_block(
		&self,
		block: &Self::Block,
		state: &mut Self::Externalities,
    ) -> Result<(), Error> {
		let instance = Instance::new(Arc::new(state.code().to_vec()))?;
		let metadata = instance.execute(&block.data)?;

		if metadata.id != block.id ||
			Some(metadata.parent_id) != block.parent_id ||
			metadata.difficulty != block.difficulty ||
			metadata.timestamp != block.timestamp
		{
			return Err(Error::InvalidMetadata)
		}
		*state.code_mut() = metadata.code.clone();

		Ok(())
    }
}
