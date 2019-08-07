#![no_std]
extern crate alloc;

mod raw_metadata;
mod generic_block;

pub use crate::raw_metadata::{RawMetadata, RawArray};
pub use crate::generic_block::GenericBlock;
