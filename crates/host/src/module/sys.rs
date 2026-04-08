use std::sync::Arc;
use wasmer::sys::BaseTunables;
use wasmer::sys::CompilerConfig;
use wasmer::sys::NativeEngineExt;
use wasmer::wasmparser;
use wasmer::Engine;
use wasmer_middlewares::Metering;

#[cfg(not(test))]
/// one hundred giga ops
pub const WASM_METERING_LIMIT: u64 = 100_000_000_000;

#[cfg(test)]
/// ten mega ops.
/// We don't want tests to run forever, and it can take several minutes for 100 giga ops to run.
pub const WASM_METERING_LIMIT: u64 = 10_000_000;

/// Configure a compiler with the metering middleware and our standard
/// nans-canonicalisation setting. Shared by both per-compiler factories below
/// so the metering policy lives in one place.
fn configure_compiler<C: CompilerConfig>(compiler: &mut C) {
    let cost_function = |_operator: &wasmparser::Operator| -> u64 { 1 };
    // @todo 100 giga-ops is totally arbitrary cutoff so we probably
    // want to make the limit configurable somehow.
    let metering = Arc::new(Metering::new(WASM_METERING_LIMIT, cost_function));
    compiler.canonicalize_nans(true);
    compiler.push_middleware(metering);
}

/// Apply our standard tunables to an engine.
///
/// Workaround for invalid memory access on iOS:
/// <https://github.com/holochain/holochain/issues/3096>
fn apply_tunables(mut engine: Engine) -> Engine {
    engine.set_tunables(BaseTunables {
        static_memory_bound: 0x4000.into(),
        static_memory_offset_guard_size: 0x1_0000,
        dynamic_memory_offset_guard_size: 0x1_0000,
    });
    engine
}

/// Build a sys engine backed by the Cranelift compiler.
#[cfg(feature = "wasmer-sys-cranelift")]
pub fn make_cranelift_engine() -> Engine {
    let mut compiler = wasmer::sys::Cranelift::default();
    configure_compiler(&mut compiler);
    apply_tunables(Engine::from(compiler))
}

/// Build a sys engine backed by the LLVM compiler.
#[cfg(feature = "wasmer-sys-llvm")]
pub fn make_llvm_engine() -> Engine {
    let mut compiler = wasmer::sys::LLVM::default();
    configure_compiler(&mut compiler);
    apply_tunables(Engine::from(compiler))
}

/// Default sys engine factory used by the test module and re-exported for
/// convenience by callers that don't care which compiler is in use.
///
/// When both compilers are enabled, prefer Cranelift as it is the development
/// default and matches the historic behaviour of this crate.
#[cfg(feature = "wasmer-sys-cranelift")]
pub fn make_engine() -> Engine {
    make_cranelift_engine()
}

#[cfg(all(feature = "wasmer-sys-llvm", not(feature = "wasmer-sys-cranelift")))]
pub fn make_engine() -> Engine {
    make_llvm_engine()
}

/// The runtime engine is used only to deserialise pre-compiled artifacts and
/// to execute function calls on the resulting instances. It does not need a
/// compiler so it is the same regardless of which sys compiler is enabled.
pub fn make_runtime_engine() -> Engine {
    Engine::headless()
}

#[cfg(test)]
mod tests {
    use super::{make_engine, make_runtime_engine};
    use crate::module::{CacheKey, ModuleCache, PlruCache};
    use std::io::Write;
    use tempfile::TempDir;
    use wasmer::Module;

    fn module_cache(filesystem_path: Option<std::path::PathBuf>) -> ModuleCache {
        ModuleCache::new(make_engine, make_runtime_engine, filesystem_path)
    }

    #[test]
    fn cache_save_to_memory_and_fs() {
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
        let tmp_fs_cache_dir = TempDir::new().unwrap();
        let module_cache = module_cache(Some(tmp_fs_cache_dir.path().to_owned()));
        assert!(module_cache
            .filesystem_path
            .clone()
            .unwrap()
            .read_dir()
            .unwrap()
            .next()
            .is_none());
        assert!(module_cache.cache.read().cache.is_empty());

        let key: CacheKey = [0u8; 32];
        let module = module_cache.get(key, &wasm).unwrap();

        // make sure module has been stored in the in-memory cache under `key`
        {
            let deserialized_cached_module = module_cache.cache.write().get_item(&key).unwrap();
            assert_eq!(*deserialized_cached_module, *module);
        }

        // make sure module has been stored in serialized filesystem cache
        {
            let serialized_module_path =
                module_cache.filesystem_path.unwrap().join(hex::encode(key));
            assert!(std::fs::metadata(serialized_module_path).is_ok());
        }
    }

    #[test]
    fn cache_save_to_memory_only() {
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
        let module_cache = module_cache(None);
        assert!(module_cache.cache.read().cache.is_empty());

        let key: CacheKey = [0u8; 32];
        let module = module_cache.get(key, &wasm).unwrap();

        // make sure module has been stored in deserialized cache under key
        {
            let deserialized_cached_module = module_cache.cache.write().get_item(&key).unwrap();
            assert_eq!(*deserialized_cached_module, *module);
        }
    }

    #[test]
    fn cache_get_from_fs() {
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
        let tmp_fs_cache_dir = TempDir::new().unwrap();
        let module_cache = module_cache(Some(tmp_fs_cache_dir.path().to_owned()));
        let key: CacheKey = [0u8; 32];

        // Build module, serialize, save directly to filesystem
        let compiler_engine = make_engine();
        let module =
            std::sync::Arc::new(Module::from_binary(&compiler_engine, wasm.as_slice()).unwrap());
        let serialized_module = module.serialize().unwrap();
        let serialized_module_path = tmp_fs_cache_dir.path().join(hex::encode(key));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&serialized_module_path)
            .unwrap();
        file.write_all(&serialized_module).unwrap();

        // make sure module can be retrieved from cache
        let module_retreived = module_cache.get(key, &wasm).unwrap();
        assert_eq!(
            *module_retreived.serialize().unwrap(),
            *module.serialize().unwrap()
        );

        // make sure module is stored in deserialized cache
        {
            let deserialized_cached_module = module_cache.cache.write().get_item(&key).unwrap();
            assert_eq!(
                *deserialized_cached_module.serialize().unwrap(),
                *module.serialize().unwrap()
            );
        }
    }

    #[test]
    fn cache_get_from_fs_corrupt() {
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

        // Bad serialized_wasm
        let bad_serialized_wasm = vec![0x00];

        let tmp_fs_cache_dir = TempDir::new().unwrap();
        let module_cache = module_cache(Some(tmp_fs_cache_dir.path().to_owned()));
        let key: CacheKey = [0u8; 32];

        // Build module, serialize, save directly to filesystem
        let serialized_module_path = tmp_fs_cache_dir.path().join(hex::encode(key));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&serialized_module_path)
            .unwrap();
        file.write_all(&bad_serialized_wasm).unwrap();

        // Module can still be retrieved from fs cache, as it has been deleted from the filesystem and re-added to the cache
        let res = module_cache.get(key, &wasm);
        assert!(res.is_ok());

        let compiler_engine = make_engine();
        let module =
            std::sync::Arc::new(Module::from_binary(&compiler_engine, wasm.as_slice()).unwrap());

        // make sure module is stored in deserialized cache
        {
            let deserialized_cached_module = module_cache.cache.write().get_item(&key).unwrap();
            assert_eq!(
                *deserialized_cached_module.serialize().unwrap(),
                *module.serialize().unwrap()
            );
        }

        // make sure module has been stored in serialized filesystem cache
        {
            let serialized_module_path =
                module_cache.filesystem_path.unwrap().join(hex::encode(key));
            assert!(std::fs::metadata(serialized_module_path).is_ok());
        }
    }
}
