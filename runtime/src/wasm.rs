use alloc::vec::Vec;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
	core::intrinsics::abort()
}

#[alloc_error_handler]
unsafe fn oom(_: core::alloc::Layout) -> ! {
	core::intrinsics::abort()
}

static mut BLOCK_ARG: Option<Vec<u8>> = None;

#[no_mangle]
unsafe extern fn write_block(len: u32) -> u32 {
	let ptr = {
		BLOCK_ARG = None;
		let mut arg = Vec::with_capacity(len as usize);
		arg.resize(len as usize, 0u8);
		BLOCK_ARG = Some(arg);
		BLOCK_ARG.as_mut().unwrap().as_mut_ptr()
	};
	ptr as u32
}

static mut CODE_ARG: Option<Vec<u8>> = None;

#[no_mangle]
unsafe extern fn write_code(len: u32) -> u32 {
	let ptr = {
		CODE_ARG = None;
		let mut arg = Vec::with_capacity(len as usize);
		arg.resize(len as usize, 0u8);
		CODE_ARG = Some(arg);
		CODE_ARG.as_mut().unwrap().as_mut_ptr()
	};
	ptr as u32
}

static mut METADATA_ARG: Option<Vec<u8>> = None;
static mut PARENT_ID_ARG: Option<Vec<u8>> = None;
static mut ID_ARG: Option<Vec<u8>> = None;

#[no_mangle]
unsafe extern fn execute() -> u32 {
	match crate::execute(
		BLOCK_ARG.as_ref().unwrap(),
		CODE_ARG.as_mut().unwrap()
	) {
		Ok(metadata) => {
			let id = metadata.id;
			ID_ARG = Some(id);
			let parent_id = metadata.parent_id;
			PARENT_ID_ARG = Some(parent_id);

			let (parent_id_ptr, parent_id_len) = {
				let len = PARENT_ID_ARG.as_ref().unwrap().len();
				let ptr = PARENT_ID_ARG.as_ref().unwrap().as_ptr();
				(ptr as u32, len as u32)
			};
			let (id_ptr, id_len) = {
				let len = ID_ARG.as_ref().unwrap().len();
				let ptr = ID_ARG.as_ref().unwrap().as_ptr();
				(ptr as u32, len as u32)
			};
			let (code_ptr, code_len) = {
				let len = CODE_ARG.as_ref().unwrap().len();
				let ptr = CODE_ARG.as_ref().unwrap().as_ptr();
				(ptr as u32, len as u32)
			};

			let metadata = metadata::RawMetadata {
				timestamp: metadata.timestamp,
				difficulty: metadata.difficulty,
				parent_id_ptr, parent_id_len,
				id_ptr, id_len,
				code_ptr, code_len,
			};
			METADATA_ARG = Some(metadata.encode());

			0
		},
		Err(_) => 1,
	}
}

#[no_mangle]
unsafe extern fn read_metadata() -> u32 {
	METADATA_ARG.as_ref().unwrap().as_ptr() as u32
}

#[no_mangle]
unsafe extern fn free() {
	BLOCK_ARG = None;
	CODE_ARG = None;
	ID_ARG = None;
	PARENT_ID_ARG = None;
	METADATA_ARG = None;
}
