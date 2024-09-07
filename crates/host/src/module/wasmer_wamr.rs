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
pub struct PreCompiledSerializedModule {}

impl PreCompiledSerializedModule {
    /// Compile a wasm binary, serialize it with wasmer's serializtion format, and write to a file.
    /// This file can later be used for contexts where JIT compilation is not possible (iOS for example).
    pub fn write(wasm: &[u8], path: PathBuf) -> Result<(), CompileError> {
        unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
    }

    /// Deserialize a previously precompiled and serialized module. While technically the `wasmer_wamr` feature flag
    /// can support deserializing a pre-compiled and serialized module, it doesn't make sense to use it, since it would be
    /// executed by an engine which will use an interpreter to anyway.
    pub fn read(path: &Path) -> Result<Module, DeserializeError> {
        unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
    }
}