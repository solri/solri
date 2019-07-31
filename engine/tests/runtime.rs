extern crate solri_engine as engine;
extern crate solri_runtime as runtime;

#[test]
fn call_runtime() {
	let instance = engine::Instance::new(runtime::WASM_BINARY).unwrap();
	let block = [6, 7, 8, 9];
	let metadata = instance.execute(&block, runtime::WASM_BINARY).unwrap();
	assert_eq!(metadata.timestamp, 1);
	assert_eq!(metadata.difficulty, 2);
	assert_eq!(metadata.parent_hash, vec![5, 6, 7, 8]);
	assert_eq!(metadata.hash, vec![1, 2, 3, 4]);
	assert_eq!(metadata.code, runtime::WASM_BINARY.to_vec());
}
