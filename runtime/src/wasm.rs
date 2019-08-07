use alloc::vec::Vec;
use metadata::{RawMetadata, RawArray};

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

			let metadata = RawMetadata {
				timestamp: metadata.timestamp,
				difficulty: metadata.difficulty,
				parent_id: RawArray {
					ptr: parent_id_ptr,
					len: parent_id_len,
				},
				id: RawArray {
					ptr: id_ptr,
					len: id_len,
				},
				code: RawArray {
					ptr: code_ptr,
					len: code_len,
				},
			};
			METADATA_ARG = Some(metadata.encode());

			0
		},
		Err(e) => {
			#[cfg(feature = "debug-error")] {
				let estr = format!("{:?}", e);
				DEBUG_ERROR_ARG = Some(estr.as_bytes().to_vec());
			}

			1
		},
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

	#[cfg(feature = "debug-error")] {
		DEBUG_LAST_ERROR_ARG = None;
		DEBUG_LAST_ERROR_METADATA_ARG = None;
	}
}

#[cfg(feature = "debug-error")]
static mut DEBUG_ERROR_ARG: Option<Vec<u8>> = None;
#[cfg(feature = "debug-error")]
static mut DEBUG_ERROR_METADATA_ARG: Option<Vec<u8>> = None;

#[cfg(feature = "debug-error")]
#[no_mangle]
unsafe extern fn debug_read_error() -> u32 {
	let error = DEBUG_ERROR_ARG.as_ref().unwrap();
	let len = error.len();
	let ptr = error.as_ptr();

	let metadata = RawArray {
		ptr: ptr as u32,
		len: len as u32,
	};
	DEBUG_ERROR_METADATA_ARG = Some(metadata.encode());

	DEBUG_ERROR_METADATA_ARG.as_ref().unwrap().as_ptr() as u32
}
