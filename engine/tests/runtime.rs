extern crate solri_engine as engine;
extern crate solri_runtime as runtime;

use parity_codec::Encode;
use runtime::{Block, Executor, Extrinsic};
use blockchain::{Block as _, SimpleBuilderExecutor};

use std::sync::Arc;

#[test]
fn call_runtime() {
	let instance = engine::Instance::new(Arc::new(runtime::WASM_BINARY.to_vec())).unwrap();
	let genesis_block = Block::genesis();
	let executor = Executor;

	let mut build_block = executor.initialize_block(&genesis_block, &mut (), 1234).unwrap();
	executor.apply_extrinsic(&mut build_block, Extrinsic::Add(5), &mut ()).unwrap();
	executor.finalize_block(&mut build_block, &mut ()).unwrap();
	let block = build_block.seal();

	let metadata = instance.execute(&block.encode()).unwrap();
	assert_eq!(metadata.timestamp, block.timestamp);
	assert_eq!(metadata.difficulty, 1);
	assert_eq!(metadata.parent_hash, block.parent_id().unwrap()[..].to_vec());
	assert_eq!(metadata.hash, block.id()[..].to_vec());
	assert_eq!(metadata.code, runtime::WASM_BINARY.to_vec());
}
