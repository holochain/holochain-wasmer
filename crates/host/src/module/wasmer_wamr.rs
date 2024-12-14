use crate::prelude::*;
use std::path::Path;
use std::sync::Arc;
use wasmer::CompileError;
use wasmer::DeserializeError;
use wasmer::Engine;
use wasmer::Module;

/// Take WASM binary and prepare a wasmer Module suitable for iOS
pub fn build_ios_module(_wasm: &[u8]) -> Result<Module, CompileError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Deserialize a previously compiled module for iOS from a file.
pub fn get_ios_module_from_file(_path: &Path) -> Result<Module, DeserializeError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Build an interpreter module from wasm bytes.
/// The interpreter used (wasm micro runtime) does not support metering
/// See tracking issue: https://github.com/bytecodealliance/wasm-micro-runtime/issues/2163
pub fn build_module(wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
    let compiler_engine = Engine::default();
    let res = Module::from_binary(&compiler_engine, wasm);
    let module = res.map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?;
    Ok(Arc::new(module))
}
