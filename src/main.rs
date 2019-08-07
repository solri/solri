use blockchain::backend::{SharedMemoryBackend, ChainQuery, ImportLock};
use blockchain::import::ImportAction;
use blockchain::{Block as BlockT, SimpleBuilderExecutor, AsExternalities};
use blockchain_network_simple::{BestDepthImporter, BestDepthStatusProducer};
use std::thread;
use std::collections::HashMap;
use clap::{App, SubCommand, AppSettings, Arg};
use parity_codec::{Encode, Decode};
use engine::CodeExternalities;

fn main() {
    let matches = App::new("Solri")
		.setting(AppSettings::SubcommandRequiredElseHelp)
		.subcommand(SubCommand::with_name("local")
					.about("Start a local test network"))
		.subcommand(SubCommand::with_name("libp2p")
					.about("Start a libp2p instance")
					.arg(Arg::with_name("port")
						 .short("p")
						 .long("port")
						 .takes_value(true)
						 .help("Port to listen on"))
					.arg(Arg::with_name("author")
						 .long("author")
						 .help("Whether to author blocks")))
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

#[derive(Clone, Eq, PartialEq, Debug, Default, Encode, Decode)]
pub struct State {
	code: Vec<u8>
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

fn local_sync() {
    let runtime_genesis_block = runtime::Block::genesis();
	let genesis_block = engine::Block {
		id: runtime_genesis_block.id()[..].to_vec(),
		parent_id: None,
		difficulty: 1,
		timestamp: runtime_genesis_block.timestamp,
		data: runtime_genesis_block.encode(),
	};
	let genesis_state = State {
		code: runtime::WASM_BINARY.to_vec(),
	};
    let (backend_build, lock_build) = (
		SharedMemoryBackend::<_, (), State>::new_with_genesis(
			genesis_block.clone(),
			genesis_state,
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
					Default::default()
				),
				ImportLock::new()
			)
		};
		let importer = BestDepthImporter::new(engine::Executor, backend.clone(), lock.clone());
		let status = BestDepthStatusProducer::new(backend.clone());
		peers.insert(peer_id, (backend, lock, importer, status));
    }
    thread::spawn(move || {
		builder_thread(backend_build, lock_build);
    });

    blockchain_network_simple::local::start_local_simple_sync(peers);
}

fn libp2p_sync(port: &str, author: bool) {
    let runtime_genesis_block = runtime::Block::genesis();
	let genesis_block = engine::Block {
		id: runtime_genesis_block.id()[..].to_vec(),
		parent_id: None,
		difficulty: 1,
		timestamp: runtime_genesis_block.timestamp,
		data: runtime_genesis_block.encode(),
	};
	let genesis_state = State {
		code: runtime::WASM_BINARY.to_vec(),
	};
    let backend = SharedMemoryBackend::<_, (), State>::new_with_genesis(
		genesis_block.clone(),
		genesis_state,
    );
    let lock = ImportLock::new();
    let importer = BestDepthImporter::new(engine::Executor, backend.clone(), lock.clone());
    let status = BestDepthStatusProducer::new(backend.clone());
    if author {
		let backend_build = backend.clone();
		let lock_build = lock.clone();
		thread::spawn(move || {
			builder_thread(backend_build, lock_build);
		});
    }
    blockchain_network_simple::libp2p::start_network_simple_sync(port, backend, lock, importer, status);
}

fn builder_thread(backend_build: SharedMemoryBackend<engine::Block, (), State>, lock: ImportLock) {
    loop {
		let head = backend_build.head();
		let runtime_executor = runtime::Executor;
		let engine_executor = engine::Executor;
		println!("Building on top of {:?}", head);

		// Build a block.
		let parent_block = runtime::Block::decode(
			&mut &backend_build.block_at(&head).unwrap().data[..]
		).unwrap();
		let pending_state = backend_build.state_at(&head).unwrap();
		assert_eq!(&pending_state.code()[..], runtime::WASM_BINARY);

		let mut unsealed_block = runtime_executor.initialize_block(
			&parent_block, ().as_externalities(), 1234
		).unwrap();

		runtime_executor.apply_extrinsic(
            &mut unsealed_block, runtime::Extrinsic::Add(1), ().as_externalities()
        ).unwrap();

		runtime_executor.finalize_block(
			&mut unsealed_block, ().as_externalities(),
		).unwrap();

		let block = unsealed_block.seal();

		// Import the built block.
		let mut build_importer = ImportAction::new(&engine_executor, &backend_build, lock.lock());
		let new_block_hash = block.id()[..].to_vec();
		build_importer.import_block(engine::Block {
			id: block.id()[..].to_vec(),
			parent_id: Some(block.parent_id().unwrap()[..].to_vec()),
			difficulty: 1,
			timestamp: block.timestamp,
			data: block.encode(),
		}).unwrap();
		build_importer.set_head(new_block_hash);
		build_importer.commit().unwrap();
    }
}
