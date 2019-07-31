use alloc::{vec, vec::Vec};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[panic_handler]
unsafe fn panic(_info: &core::panic::PanicInfo) -> ! {
	unsafe { core::intrinsics::abort() }
}

#[alloc_error_handler]
unsafe fn oom(_: core::alloc::Layout) -> ! {
	unsafe { core::intrinsics::abort() }
}

static mut BLOCK_ARG: Option<Vec<u8>> = None;

#[no_mangle]
unsafe extern fn write_block(len: u32) -> u32 {
	let ptr = unsafe {
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
	let ptr = unsafe {
		CODE_ARG = None;
		let mut arg = Vec::with_capacity(len as usize);
		arg.resize(len as usize, 0u8);
		CODE_ARG = Some(arg);
		CODE_ARG.as_mut().unwrap().as_mut_ptr()
	};
	ptr as u32
}

static mut METADATA_ARG: Option<Vec<u8>> = None;
static mut PARENT_HASH_ARG: Option<Vec<u8>> = None;
static mut HASH_ARG: Option<Vec<u8>> = None;

#[no_mangle]
unsafe extern fn execute() -> u32 {
	let hash = vec![1, 2, 3, 4];
	unsafe { HASH_ARG = Some(hash); }
	let parent_hash = vec![5, 6, 7, 8];
	unsafe { PARENT_HASH_ARG = Some(parent_hash); }

	let (parent_hash_ptr, parent_hash_len) = unsafe {
		let len = PARENT_HASH_ARG.as_ref().unwrap().len();
		let ptr = PARENT_HASH_ARG.as_ref().unwrap().as_ptr();
		(ptr as u32, len as u32)
	};
	let (hash_ptr, hash_len) = unsafe {
		let len = HASH_ARG.as_ref().unwrap().len();
		let ptr = HASH_ARG.as_ref().unwrap().as_ptr();
		(ptr as u32, len as u32)
	};
	let (code_ptr, code_len) = unsafe {
		let len = CODE_ARG.as_ref().unwrap().len();
		let ptr = CODE_ARG.as_ref().unwrap().as_ptr();
		(ptr as u32, len as u32)
	};

	let metadata = metadata::RawMetadata {
		timestamp: 1,
		difficulty: 2,
		parent_hash_ptr, parent_hash_len,
		hash_ptr, hash_len,
		code_ptr, code_len,
	};
	unsafe { METADATA_ARG = Some(metadata.encode()); }

	0
}

#[no_mangle]
unsafe extern fn read_metadata() -> u32 {
	unsafe { METADATA_ARG.as_ref().unwrap().as_ptr() as u32 }
}

#[no_mangle]
unsafe extern fn free() {
	unsafe {
		BLOCK_ARG = None;
		CODE_ARG = None;
		HASH_ARG = None;
		PARENT_HASH_ARG = None;
		METADATA_ARG = None;
	}
}
