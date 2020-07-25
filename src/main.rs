use blockchain::backend::{
	SharedMemoryBackend, ChainQuery, ImportLock, Store,
	Operation, SharedCommittable
};
use blockchain::{Block as BlockT, ExtrinsicBuilder, AsExternalities, BlockExecutor};
use blockchain::import::{BlockImporter, ImportAction};
use blockchain_network::sync::{BestDepthError, BestDepthStatusProducer};
use std::thread;
use std::collections::HashMap;
use clap::{App, SubCommand, AppSettings, Arg};
use parity_codec::Decode;
use runtime::TrieExternalities;
use engine::{GenericBlock, CodeExternalities};
use bm::{InMemoryBackend, ReadBackend, WriteBackend, DynBackend};

fn main() {
	let matches = App::new("Solri")
		.setting(AppSettings::SubcommandRequiredElseHelp)
		.subcommand(
			SubCommand::with_name("local")
				.about("Start a local test network")
		)
		.subcommand(
			SubCommand::with_name("libp2p")
				.about("Start a libp2p instance")
				.arg(
					Arg::with_name("port")
						.short("p")
						.long("port")
						.takes_value(true)
						.help("Port to listen on")
				)
				.arg(
					Arg::with_name("author")
						.long("author")
						.help("Whether to author blocks")
				)
		)
		.get_matches();

	if let Some(_) = matches.subcommand_matches("local") {
		local_sync();
		return
	}

	if let Some(matches) = matches.subcommand_matches("libp2p") {
		let port = matches.value_of("port").unwrap_or("37365");
		let author = matches.is_present("author");
		libp2p_sync(port, author);
		return
	}
}

#[derive(Debug)]
pub enum Error {
	StateNotAvailable,
	OutdatedRuntime,
	Backend(Box<dyn std::error::Error>),
	NativeExecutor(Box<dyn std::error::Error>),
}

#[derive(Clone)]
pub struct State {
	code: Vec<u8>,
	trie: Option<DynBackend<InMemoryBackend<runtime::Construct>>>,
}

impl CodeExternalities for State {
	fn code(&self) -> &Vec<u8> { &self.code }
	fn code_mut(&mut self) -> &mut Vec<u8> { &mut self.code }
}
impl AsExternalities<dyn CodeExternalities> for State {
	fn as_externalities(&mut self) -> &mut (dyn CodeExternalities + 'static) {
		self
	}
}

impl TrieExternalities for State {
	fn db(&self) -> &dyn ReadBackend<Construct=runtime::Construct, Error=()> {
		self.trie.as_ref().expect("Trie state is not available")
	}

	fn db_mut(&mut self) -> &mut dyn WriteBackend<Construct=runtime::Construct, Error=()> {
		self.trie.as_mut().expect("Trie state is not available")
	}
}
impl AsExternalities<dyn TrieExternalities> for State {
	fn as_externalities(&mut self) -> &mut (dyn TrieExternalities + 'static) {
		self
	}
}

pub struct BestDepthImporter<Ba> {
	backend: Ba,
	import_lock: ImportLock,
	native: runtime::Executor,
	generic: engine::Executor,
}

impl<Ba> BestDepthImporter<Ba> {
	pub fn new(backend: Ba, import_lock: ImportLock) -> Self {
		Self {
			backend, import_lock,
			native: runtime::Executor,
			generic: engine::Executor,
		}
	}
}

impl<Ba: ChainQuery + Store<Block=GenericBlock, State=State, Auxiliary=()>> BlockImporter for BestDepthImporter<Ba> where
	Ba: SharedCommittable<Operation=Operation<GenericBlock, State, ()>>,
{
	type Block = GenericBlock;
	type Error = BestDepthError;

	fn import_block(&mut self, block: GenericBlock) -> Result<(), Self::Error> {
		let mut importer = ImportAction::new(
			&self.backend,
			self.import_lock.lock()
		);
		let new_hash = block.id();
		let (current_best_depth, current_best_state, new_depth) = {
			let backend = importer.backend();
			let current_best_hash = backend.head();
			let current_best_depth = backend.depth_at(&current_best_hash)
				.expect("Best block depth hash cannot fail");
			let current_best_state = backend.state_at(&current_best_hash)
				.expect("Best block depth state cannot fail");
			let new_parent_depth = block.parent_id()
				.map(|parent_hash| {
					backend.depth_at(&parent_hash).unwrap()
				})
				.unwrap_or(0);
			(current_best_depth, current_best_state, new_parent_depth + 1)
		};

		let mut pending_state = current_best_state;
		if pending_state.trie.is_some() && &pending_state.code[..] == runtime::WASM_BINARY {
			let decoded = runtime::Block::decode(&mut &block.data[..])
				.ok_or(BestDepthError::Executor(Box::new(engine::Error::ExecutionFailed)))?;
			self.native.execute_block(&decoded, &mut pending_state)
				.map_err(|e| BestDepthError::Executor(Box::new(e)))?;
		} else {
			self.generic.execute_block(&block, &mut pending_state)
				.map_err(|e| BestDepthError::Executor(Box::new(e)))?;
			pending_state.trie = None;
		}

		importer.import_block(block, pending_state);
		if new_depth > current_best_depth {
			importer.set_head(new_hash);
		}
		importer.commit().map_err(|e| BestDepthError::Backend(Box::new(e)))?;

		Ok(())
	}
}

fn local_sync() {
	let runtime_genesis_block = runtime::Block::genesis();
	let genesis_block: engine::GenericBlock = runtime_genesis_block.clone().into();
	let genesis_state = State {
		code: runtime::WASM_BINARY.to_vec(),
		trie: Some(Default::default()),
	};
	let (backend_build, lock_build) = (
		SharedMemoryBackend::<_, (), State>::new_with_genesis(
			genesis_block.clone(),
			genesis_state.clone(),
		),
		ImportLock::new()
	);
	let mut peers = HashMap::new();
	for peer_id in 0..4 {
		let (backend, lock) = if peer_id == 0 {
			(backend_build.clone(), lock_build.clone())
		} else {
			(
				SharedMemoryBackend::<_, (), State>::new_with_genesis(
					genesis_block.clone(),
					genesis_state.clone(),
				),
				ImportLock::new()
			)
		};
		let importer = BestDepthImporter::new(backend.clone(), lock.clone());
		let status = BestDepthStatusProducer::new(backend.clone());
		peers.insert(peer_id, (backend, lock, importer, status));
	}
	thread::spawn(move || {
		builder_thread(backend_build, lock_build);
	});

	blockchain_network_local::start_local_simple_sync(peers);
}

fn libp2p_sync(port: &str, author: bool) {
	let runtime_genesis_block = runtime::Block::genesis();
	let genesis_block: engine::GenericBlock = runtime_genesis_block.clone().into();
	let genesis_state = State {
		code: runtime::WASM_BINARY.to_vec(),
		trie: Some(Default::default()),
	};
	let backend = SharedMemoryBackend::<_, (), State>::new_with_genesis(
		genesis_block.clone(),
		genesis_state,
	);
	let lock = ImportLock::new();
	let importer = BestDepthImporter::new(backend.clone(), lock.clone());
	let status = BestDepthStatusProducer::new(backend.clone());
	if author {
		let backend_build = backend.clone();
		let lock_build = lock.clone();
		thread::spawn(move || {
			builder_thread(backend_build, lock_build);
		});
	}
	blockchain_network_libp2p::start_network_simple_sync(port, backend, lock, importer, status);
}

fn builder_thread(backend_build: SharedMemoryBackend<engine::GenericBlock, (), State>, lock: ImportLock) {
	loop {
		build_one(&backend_build, &lock).unwrap();
		std::thread::sleep(std::time::Duration::new(1, 0));
	}
}

fn build_one<Ba>(backend_build: &Ba, lock: &ImportLock) -> Result<(), Error> where
	Ba: Store<Block=engine::GenericBlock, State=State, Auxiliary=()> + ChainQuery + ?Sized,
	Ba: SharedCommittable<Operation=Operation<engine::GenericBlock, State, ()>>,
{
	let head = backend_build.head();
	let runtime_executor = runtime::Executor;
	println!("Building on top of {:?}", head);

	// Build a block.
	let parent_block = runtime::Block::decode(
		&mut &backend_build.block_at(&head).map_err(|e| Error::Backend(Box::new(e)))?.data[..]
	).ok_or(Error::OutdatedRuntime)?;
	let mut pending_state = backend_build.state_at(&head).map_err(|e| Error::Backend(Box::new(e)))?;
	if &pending_state.code()[..] != runtime::WASM_BINARY {
		return Err(Error::OutdatedRuntime)
	}
	if pending_state.trie.is_none() {
		return Err(Error::StateNotAvailable)
	}

	let mut unsealed_block = runtime_executor.initialize_block(
		&parent_block, &mut pending_state, 1234
	).map_err(|e| Error::NativeExecutor(Box::new(e)))?;

	runtime_executor.apply_extrinsic(
		&mut unsealed_block, runtime::Extrinsic::Add(1), &mut pending_state
	).map_err(|e| Error::NativeExecutor(Box::new(e)))?;

	runtime_executor.finalize_block(
		&mut unsealed_block, &mut pending_state
	).map_err(|e| Error::NativeExecutor(Box::new(e)))?;

	let block = unsealed_block.seal();

	// Import the built block.
	let mut build_importer = ImportAction::<Ba>::new(
		backend_build, lock.lock()
	);
	let new_block_hash = block.id()[..].to_vec();
	build_importer.import_block(block.clone().into(), pending_state);
	build_importer.set_head(new_block_hash);
	build_importer.commit().map_err(|e| Error::Backend(Box::new(e)))?;

	Ok(())
}
