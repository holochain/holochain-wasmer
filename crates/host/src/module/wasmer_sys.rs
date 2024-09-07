use std::path::Path;
use std::sync::Arc;
use tracing::info;
use wasmer::sys::BaseTunables;
use wasmer::sys::CompilerConfig;
use wasmer::wasmparser;
use wasmer::CompileError;
use wasmer::Cranelift;
use wasmer::DeserializeError;
use wasmer::Engine;
use wasmer::Module;
use wasmer::NativeEngineExt;
use wasmer::Store;
use wasmer_middlewares::Metering;

#[cfg(not(test))]
/// one hundred giga ops
pub const WASM_METERING_LIMIT: u64 = 100_000_000_000;

#[cfg(test)]
/// ten mega ops.
/// We don't want tests to run forever, and it can take several minutes for 100 giga ops to run.
pub const WASM_METERING_LIMIT: u64 = 10_000_000;

/// Generate an engine with a wasm compiler
/// and Metering (use limits) in place.
pub fn make_engine() -> Engine {
    let cost_function = |_operator: &wasmparser::Operator| -> u64 { 1 };
    // @todo 100 giga-ops is totally arbitrary cutoff so we probably
    // want to make the limit configurable somehow.
    let metering = Arc::new(Metering::new(WASM_METERING_LIMIT, cost_function));
    // the only place where the wasm compiler engine is set
    let mut compiler = Cranelift::default();
    compiler.canonicalize_nans(true).push_middleware(metering);

    // Workaround for invalid memory access on iOS.
    // https://github.com/holochain/holochain/issues/3096
    let mut engine = Engine::from(compiler);
    engine.set_tunables(BaseTunables {
        static_memory_bound: 0x4000.into(),
        static_memory_offset_guard_size: 0x1_0000,
        dynamic_memory_offset_guard_size: 0x1_0000,
    });

    engine
}

pub fn make_runtime_engine() -> Engine {
    Engine::headless()
}


/// Compile a wasm binary, serialize it with wasmer's serializtion format, and write to a file.
/// This file can later be used for contexts where JIT compilation is not possible (iOS for example).
pub fn write_precompiled_serialized_module_to_file() -> Result<(), CompileError> {
    let compiler_engine = make_engine();
    let store = Store::new(compiler_engine);
    Module::from_binary(&store, wasm)?.serialize_to_file(path.clone())?;
    Ok(())
}

/// Deserialize a previously precompiled and serialized module
pub fn read_precompiled_serialized_module_from_file() -> Result<Module, DeserializeError> {
    let engine = Engine::headless();
    unsafe { Module::deserialize_from_file(&engine, path) }
}
