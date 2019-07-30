use clap::{App, Arg};
use std::fs::File;
use std::io::Write;

fn main() {
	let matches = App::new("Solri")
		.arg(Arg::with_name("file")
			 .takes_value(true))
		.get_matches();

	let file_name = matches.value_of("file").unwrap();
	let mut file = File::create(file_name).unwrap();
	file.write_all(runtime::WASM_BINARY).unwrap();
}
