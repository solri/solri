// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use wasm_builder_runner::{build_current_project_with_features, WasmBuilderSource};
use std::env;

fn main() {
	let mut features = Vec::new();
	if env::var("CARGO_FEATURE_DEBUG_ERROR").is_ok() {
		features.push("debug-error");
	}

	build_current_project_with_features(
		"wasm_binary.rs",
		WasmBuilderSource::CratesOrPath {
			path: "../utils/wasm-builder",
			version: "1.0.4",
		},
		&features,
	);
}
