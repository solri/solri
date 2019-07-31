#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]
#![cfg_attr(not(feature = "std"), feature(core_intrinsics))]

extern crate alloc;

#[cfg(all(not(feature = "std"), target_arch = "wasm32"))]
mod wasm;

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use alloc::{vec, vec::Vec};

pub enum Error {
	InvalidBlock,
}

#[derive(Debug)]
pub struct Metadata<'a> {
	pub timestamp: u64,
	pub difficulty: u64,
	pub parent_hash: Vec<u8>,
	pub hash: Vec<u8>,
	pub code: &'a mut Vec<u8>,
}

pub fn execute<'a>(_block: &[u8], code: &'a mut Vec<u8>) -> Result<Metadata<'a>, Error> {
	Ok(Metadata {
		timestamp: 1,
		difficulty: 2,
		parent_hash: vec![5, 6, 7, 8],
		hash: vec![1, 2, 3, 4],
		code: code,
	})
}
