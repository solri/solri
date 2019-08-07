use crate::{Error, Instance};
use std::sync::Arc;
use blockchain::BlockExecutor;
use metadata::GenericBlock;

pub trait CodeExternalities {
	fn code(&self) -> &Vec<u8>;
	fn code_mut(&mut self) -> &mut Vec<u8>;
}

#[derive(Default)]
pub struct Executor;

impl BlockExecutor for Executor {
    type Error = Error;
    type Block = GenericBlock;
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
