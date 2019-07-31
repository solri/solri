#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]
#![cfg_attr(not(feature = "std"), feature(core_intrinsics))]

extern crate alloc;

#[cfg(all(not(feature = "std"), target_arch = "wasm32"))]
mod wasm;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
