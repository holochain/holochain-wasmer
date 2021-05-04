use holochain_wasmer_common::WasmError;
use std::path::PathBuf;
use wasmer::ImportObject;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
use wasmer::JIT;
use wasmer_compiler_singlepass::Singlepass;

pub fn module<P: Into<PathBuf>>(
    cache_key_bytes: &[u8],
    wasm: &[u8],
    cache_path: Option<P>,
) -> Result<Module, WasmError> {
    // let mut cache = MemoryFallbackFileSystemCache::new(cache_path)
    //     .map_err(|e| WasmError::Compile(e.to_string()))?;
    // let key = WasmHash::generate(cache_key_bytes);

    // Ok(match cache.load(key) {
    //     Ok(module) => module,
    //     Err(_) => {
    //         let module = compile_with(wasm, &Singlepass::new())
    //             .map_err(|e| WasmError::Compile(e.to_string()))?;
    //         cache
    //             .store(key, module.clone())
    //             .expect("could not store compiled wasm");
    //         module
    //     }
    // })
    let engine = JIT::new(Singlepass::new()).engine();
    let store = Store::new(&engine);
    let module = Module::new(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
    Ok(module)
}

pub fn instance<P: Into<PathBuf>>(
    cache_key_bytes: &[u8],
    wasm: &[u8],
    wasm_imports: &ImportObject,
    cache_path: Option<P>,
) -> Result<Instance, WasmError> {
    let mut instance = Instance::new(&module(cache_key_bytes, wasm, cache_path)?, &wasm_imports)
        .map_err(|e| WasmError::Compile(e.to_string()))?;
    Ok(instance)
}
