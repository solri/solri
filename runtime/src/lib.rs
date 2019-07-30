#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(all(not(feature = "std"), target_arch = "wasm32"))]
#[panic_handler]
#[no_mangle]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
	unsafe { core::hint::unreachable_unchecked() }
}
