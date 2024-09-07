use wasmer::CompileError;
use wasmer::DeserializeError;
use wasmer::Engine;
use wasmer::Module;
use std::path::PathBuf;

/// Generate an engine with a wasm interpreter
/// The interpreter used (wasm micro runtime) does not support metering
/// See tracking issue: https://github.com/bytecodealliance/wasm-micro-runtime/issues/2163
pub fn make_engine() -> Engine {
    Engine::default()
}

pub fn make_runtime_engine() -> Engine {
    Engine::default()
}

/// Compile a wasm binary, serialize it with wasmer's serializtion format, and write to a file.
/// This file can later be used for contexts where JIT compilation is not possible (iOS for example).
pub fn write_precompiled_serialized_module_to_file(wasm: &[u8], path: PathBuf) -> Result<(), PreCompiledSerilializedModuleError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Deserialize a previously precompiled and serialized module. 
/// Even though the `wasmer_wamr` feature flag supports deserializing a pre-compiled and serialized module,
/// it doesn't make sense to use a pre-compiled module as it would be executed by the interpreter engine anyway.
pub fn read_precompiled_serialized_module_from_file(path: &Path) -> Result<Module, DeserializeError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}
