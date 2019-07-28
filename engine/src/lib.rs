use wasmi::RuntimeValue;
use core::mem;

pub enum Error {
	Interpreter(wasmi::Error),
	InstanceHasStart,
	InstanceMemoryNotExported,
	InvalidFunctionSignature,
	InvalidMetadata,
	ExecutionFailed,
}

impl From<wasmi::Error> for Error {
	fn from(err: wasmi::Error) -> Error {
		Error::Interpreter(err)
	}
}

pub struct MetadataPtr {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_hash_ptr: u32,
	pub parent_hash_len: u32,
	pub hash_ptr: u32,
	pub hash_len: u32,
	pub code_ptr: u32,
	pub code_len: u32,
}

impl MetadataPtr {
	pub fn bytes_len() -> usize {
		mem::size_of::<u64>() + // timestamp
			mem::size_of::<u64>() + // difficulty
			mem::size_of::<u32>() + // parent_hash_ptr
			mem::size_of::<u32>() + // parent_hash_len
			mem::size_of::<u32>() + // hash_ptr
			mem::size_of::<u32>() + // hash_len
			mem::size_of::<u32>() + // code_ptr
			mem::size_of::<u32>() // code_len
	}

	pub fn decode(bytes: &[u8]) -> Option<Self> {
		fn decode_u64(bytes: &[u8]) -> Option<u64> {
			let mut arr = 0u64.to_le_bytes();
			if arr.len() != bytes.len() {
				return None
			}
			arr.copy_from_slice(bytes);
			Some(u64::from_le_bytes(arr))
		}

		fn decode_u32(bytes: &[u8]) -> Option<u32> {
			let mut arr = 0u32.to_le_bytes();
			if arr.len() != bytes.len() {
				return None
			}
			arr.copy_from_slice(bytes);
			Some(u32::from_le_bytes(arr))
		}

		Some(MetadataPtr {
			timestamp: decode_u64(&bytes[0..8])?,
			difficulty: decode_u64(&bytes[8..16])?,
			parent_hash_ptr: decode_u32(&bytes[16..20])?,
			parent_hash_len: decode_u32(&bytes[20..24])?,
			hash_ptr: decode_u32(&bytes[24..28])?,
			hash_len: decode_u32(&bytes[28..32])?,
			code_ptr: decode_u32(&bytes[32..36])?,
			code_len: decode_u32(&bytes[36..40])?,
		})
	}
}

pub struct MetadataInfo {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_hash: Vec<u8>,
	pub hash: Vec<u8>,
	pub code: Vec<u8>,
}

pub struct Instance {
	instance: wasmi::ModuleRef,
	memory: wasmi::MemoryRef,
}

impl Instance {
	pub fn new(code: &[u8]) -> Result<Self, Error> {
		let module = wasmi::Module::from_buffer(code)?;
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
		Ok(Self { instance, memory })
	}

	pub fn execute(&self, block: &[u8], code: &[u8]) -> Result<MetadataInfo, Error> {
		self.call_write_block(block)?;
		self.call_write_code(code)?;
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

	fn call_read_metadata(&self) -> Result<MetadataInfo, Error> {
		match self.instance.invoke_export(
			"read_metadata",
			&[],
			&mut wasmi::NopExternals,
		)? {
			Some(RuntimeValue::I32(ptr)) => {
				let len = MetadataPtr::bytes_len();
				let bytes = self.memory.get(ptr as u32, len)?;
				let metadata_ptr = MetadataPtr::decode(&bytes)
					.ok_or(Error::InvalidMetadata)?;
				let parent_hash = self.memory.get(
					metadata_ptr.parent_hash_ptr,
					metadata_ptr.parent_hash_len as usize
				)?;
				let hash = self.memory.get(
					metadata_ptr.hash_ptr,
					metadata_ptr.hash_len as usize
				)?;
				let code = self.memory.get(
					metadata_ptr.code_ptr,
					metadata_ptr.code_len as usize
				)?;
				Ok(MetadataInfo {
					timestamp: metadata_ptr.timestamp,
					difficulty: metadata_ptr.difficulty,
					parent_hash,
					hash,
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
