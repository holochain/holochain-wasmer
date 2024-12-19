use std::path::Path;
use std::sync::Arc;
use tracing::info;
use wasmer::sys::BaseTunables;
use wasmer::sys::CompilerConfig;
use wasmer::wasmparser;
use wasmer::CompileError;
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
pub(crate) fn make_engine() -> Engine {
    let cost_function = |_operator: &wasmparser::Operator| -> u64 { 1 };
    // @todo 100 giga-ops is totally arbitrary cutoff so we probably
    // want to make the limit configurable somehow.
    let metering = Arc::new(Metering::new(WASM_METERING_LIMIT, cost_function));

    // the only place where the wasm compiler engine is set
    #[cfg(feature = "wasmer_sys_dev")]
    let mut compiler = wasmer::Cranelift::default();
    #[cfg(feature = "wasmer_sys_prod")]
    let mut compiler = wasmer::LLVM::default();

    compiler.canonicalize_nans(true);
    compiler.push_middleware(metering);

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

pub(crate) fn make_runtime_engine() -> Engine {
    Engine::headless()
}

/// Build an interpreter module from wasm bytes.
pub fn build_module(_wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
    unimplemented!("The feature flag 'wasmer_wamr' must be enabled to support building a Module directly. Please use the ModuleCache instead.");
}

/// Take WASM binary and prepare a wasmer Module suitable for iOS
pub fn build_ios_module(wasm: &[u8]) -> Result<Module, CompileError> {
    info!(
        "Found wasm and was instructed to serialize it for ios in wasmer format, doing so now..."
    );
    let compiler_engine = make_engine();
    let store = Store::new(compiler_engine);
    Module::from_binary(&store, wasm)
}

/// Deserialize a previously compiled module for iOS from a file.
pub fn get_ios_module_from_file(path: &Path) -> Result<Module, DeserializeError> {
    let engine = Engine::headless();
    unsafe { Module::deserialize_from_file(&engine, path) }
}

#[cfg(test)]
mod tests {
    use crate::module::{CacheKey, ModuleCache, PlruCache};

    #[test]
    fn cache_test() {
        // simple example wasm taken from wasmer docs
        // https://docs.rs/wasmer/latest/wasmer/struct.Module.html#example
        let wasm: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7f,
            0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x0b, 0x01, 0x07, 0x61, 0x64, 0x64, 0x5f,
            0x6f, 0x6e, 0x65, 0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x41, 0x01,
            0x6a, 0x0b, 0x00, 0x1a, 0x04, 0x6e, 0x61, 0x6d, 0x65, 0x01, 0x0a, 0x01, 0x00, 0x07,
            0x61, 0x64, 0x64, 0x5f, 0x6f, 0x6e, 0x65, 0x02, 0x07, 0x01, 0x00, 0x01, 0x00, 0x02,
            0x70, 0x30,
        ];
        let module_cache = ModuleCache::new(None);
        assert!(module_cache.serialized_module_cache.read().cache.is_empty());
        assert!(module_cache
            .deserialized_module_cache
            .read()
            .cache
            .is_empty());

        let key: CacheKey = [0u8; 32];
        let module = module_cache.get(key, &wasm).unwrap();

        // make sure module has been stored in serialized cache under key
        {
            let serialized_cached_module =
                module_cache.serialized_module_cache.write().get_item(&key);
            assert!(serialized_cached_module.is_some());
        }
        // make sure module has been stored in deserialized cache under key
        {
            let deserialized_cached_module = module_cache
                .deserialized_module_cache
                .write()
                .get_item(&key)
                .unwrap();
            assert_eq!(*deserialized_cached_module, *module);
        }
    }
}
