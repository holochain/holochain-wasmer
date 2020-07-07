use crate::import::free_context_data;
use crate::memory_cache::MemoryFallbackFileSystemCache;
use holochain_wasmer_common::WasmError;
use std::path::PathBuf;
use wasmer_runtime::cache::Cache;
use wasmer_runtime::cache::WasmHash;
use wasmer_runtime::compile;
use wasmer_runtime::ImportObject;
use wasmer_runtime::Instance;
use wasmer_runtime::Module;

pub fn module<P: Into<PathBuf>>(
    cache_key_bytes: &[u8],
    wasm: &[u8],
    cache_path: Option<P>,
) -> Result<Module, WasmError> {
    let mut cache = MemoryFallbackFileSystemCache::new(cache_path)
        .map_err(|e| WasmError::Compile(e.to_string()))?;
    let key = WasmHash::generate(cache_key_bytes);

    Ok(match cache.load(key) {
        Ok(module) => module,
        Err(_) => {
            let module = compile(wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
            cache
                .store(key, module.clone())
                .expect("could not store compiled wasm");
            module
        }
    })
}

pub fn instantiate<P: Into<PathBuf>>(
    cache_key_bytes: &[u8],
    wasm: &[u8],
    wasm_imports: &ImportObject,
    cache_path: Option<P>,
) -> Result<Instance, WasmError> {
    let mut instance = module(cache_key_bytes, wasm, cache_path)?
        .instantiate(wasm_imports)
        .map_err(|e| WasmError::Compile(e.to_string()))?;
    instance.context_mut().data_finalizer = Some(free_context_data);
    Ok(instance)
}
