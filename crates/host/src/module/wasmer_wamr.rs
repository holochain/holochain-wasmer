use std::path::Path;
use wasmer::CompileError;
use wasmer::DeserializeError;
use wasmer::Engine;
use wasmer::Module;

/// Generate an engine with a wasm interpreter
/// The interpreter used (wasm micro runtime) does not support metering
/// See tracking issue: https://github.com/bytecodealliance/wasm-micro-runtime/issues/2163
pub fn make_engine() -> Engine {
    Engine::default()
}

pub fn make_runtime_engine() -> Engine {
    Engine::default()
}

/// Take WASM binary and prepare a wasmer Module suitable for iOS
pub fn build_ios_module(_wasm: &[u8]) -> Result<Module, CompileError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Deserialize a previously compiled module for iOS from a file.
pub fn get_ios_module_from_file(_path: &Path) -> Result<Module, DeserializeError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}
