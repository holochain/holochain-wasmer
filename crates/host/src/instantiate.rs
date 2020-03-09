use crate::memory_cache::MemoryFallbackFileSystemCache;
use holochain_wasmer_common::WasmError;
use wasmer_runtime::cache::Cache;
use wasmer_runtime::cache::WasmHash;
use wasmer_runtime::compile;
use wasmer_runtime::ImportObject;
use wasmer_runtime::Instance;

pub fn instantiate(
    cache_key_bytes: &[u8],
    wasm: &Vec<u8>,
    wasm_imports: &ImportObject,
) -> Result<Instance, WasmError> {
    // @TODO figure out how best to use the file system
    let mut cache = MemoryFallbackFileSystemCache::new::<String>(None)
        .map_err(|e| WasmError::Compile(e.to_string()))?;
    let key = WasmHash::generate(cache_key_bytes);

    let module = match cache.load(key) {
        Ok(module) => module,
        Err(_) => {
            let module = compile(wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
            cache
                .store(key, module.clone())
                .expect("could not store compiled wasm");
            module
        }
    };
    let instance = module
        .instantiate(wasm_imports)
        .map_err(|e| WasmError::Compile(e.to_string()))?;

    Ok(instance)
}
