mod executor;

pub use self::executor::{Executor, CodeExternalities};
pub use metadata::GenericBlock;

use wasmi::RuntimeValue;
use metadata::RawMetadata;
use std::sync::Arc;
use std::collections::HashMap;
use std::error as stderror;

#[derive(Debug)]
pub struct Metadata {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_id: Vec<u8>,
	pub id: Vec<u8>,
	pub code: Vec<u8>,
}

#[derive(Debug)]
pub enum Error {
	Interpreter(wasmi::Error),
	InstanceHasStart,
	InstanceMemoryNotExported,
	InvalidFunctionSignature,
	InvalidMetadata,
	ExecutionFailed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self)
    }
}

impl stderror::Error for Error { }

impl From<Error> for blockchain::import::Error {
    fn from(error: Error) -> Self {
		blockchain::import::Error::Executor(Box::new(error))
    }
}

impl From<wasmi::Error> for Error {
	fn from(err: wasmi::Error) -> Error {
		Error::Interpreter(err)
	}
}

pub struct Instance {
	code: Arc<Vec<u8>>,
	instance: wasmi::ModuleRef,
	memory: wasmi::MemoryRef,
}

impl Instance {
	pub fn new(code: Arc<Vec<u8>>) -> Result<Self, Error> {
		let module = wasmi::Module::from_buffer(code.as_ref())?;
		let instance = wasmi::ModuleInstance::new(
			&module,
			&wasmi::ImportsBuilder::default()
		)?;
		if instance.has_start() {
			return Err(Error::InstanceHasStart)
		}
		let instance = instance.assert_no_start();
		let memory = instance.export_by_name("memory")
			.ok_or_else(|| Error::InstanceMemoryNotExported)?
			.as_memory()
			.ok_or_else(|| Error::InstanceMemoryNotExported)?
			.clone();
		Ok(Self { instance, memory, code })
	}

	pub fn execute(&self, block: &[u8]) -> Result<Metadata, Error> {
		self.call_write_block(block)?;
		self.call_write_code(self.code.as_ref())?;
		self.call_execute()?;
		let metadata = self.call_read_metadata()?;
		self.call_free()?;
		Ok(metadata)
	}

	fn call_write_block(&self, block: &[u8]) -> Result<(), Error> {
		match self.instance.invoke_export(
			"write_block",
			&[RuntimeValue::I32(block.len() as i32)],
			&mut wasmi::NopExternals,
		)? {
			Some(RuntimeValue::I32(ptr)) => {
				self.memory.set(ptr as u32, block)?;
				Ok(())
			},
			_ => return Err(Error::InvalidFunctionSignature),
		}
	}

	fn call_write_code(&self, code: &[u8]) -> Result<(), Error> {
		match self.instance.invoke_export(
			"write_code",
			&[RuntimeValue::I32(code.len() as i32)],
			&mut wasmi::NopExternals,
		)? {
			Some(RuntimeValue::I32(ptr)) => {
				self.memory.set(ptr as u32, code)?;
				Ok(())
			},
			_ => return Err(Error::InvalidFunctionSignature),
		}
	}

	fn call_read_metadata(&self) -> Result<Metadata, Error> {
		match self.instance.invoke_export(
			"read_metadata",
			&[],
			&mut wasmi::NopExternals,
		)? {
			Some(RuntimeValue::I32(ptr)) => {
				let len = RawMetadata::bytes_len();
				let bytes = self.memory.get(ptr as u32, len)?;
				let metadata_ptr = RawMetadata::decode(&bytes)
					.ok_or(Error::InvalidMetadata)?;
				let parent_id = self.memory.get(
					metadata_ptr.parent_id_ptr,
					metadata_ptr.parent_id_len as usize
				)?;
				let id = self.memory.get(
					metadata_ptr.id_ptr,
					metadata_ptr.id_len as usize
				)?;
				let code = self.memory.get(
					metadata_ptr.code_ptr,
					metadata_ptr.code_len as usize
				)?;
				Ok(Metadata {
					timestamp: metadata_ptr.timestamp,
					difficulty: metadata_ptr.difficulty,
					parent_id,
					id,
					code,
				})
			},
			_ => return Err(Error::InvalidFunctionSignature),
		}
	}

	fn call_execute(&self) -> Result<(), Error> {
		match self.instance.invoke_export(
			"execute",
			&[],
			&mut wasmi::NopExternals,
		)? {
			Some(RuntimeValue::I32(status)) => {
				if status == 0 {
					Ok(())
				} else {
					Err(Error::ExecutionFailed)
				}
			},
			_ => Err(Error::InvalidFunctionSignature),
		}
	}

	fn call_free(&self) -> Result<(), Error> {
		match self.instance.invoke_export(
			"free",
			&[],
			&mut wasmi::NopExternals,
		)? {
			None => Ok(()),
			_ => Err(Error::InvalidFunctionSignature),
		}
	}
}

#[derive(Default)]
pub struct Cache {
	cache: HashMap<Vec<u8>, Instance>,
}

impl Cache {
	pub fn execute(&mut self, block: &[u8], code: &[u8]) -> Result<Metadata, Error> {
		let code = code.to_vec();
		self.cache.entry(code.clone()).or_insert(Instance::new(Arc::new(code))?)
			.execute(block)
	}
}
