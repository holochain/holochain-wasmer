use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use wasmer::Module;
use wasmer::Store;
use wasmer::JIT;
use wasmer_compiler_singlepass::Singlepass;

pub struct ModuleCache(HashMap<[u8; 32], Module>);

pub static MODULE_CACHE: Lazy<Mutex<ModuleCache>> = Lazy::new(|| Mutex::new(ModuleCache::new()));

impl ModuleCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Module, WasmError> {
        match self.0.get(&key) {
            // Cloning a wasmer module is cheap.
            Some(module) => Ok(module.clone()),
            None => {
                let engine = JIT::new(Singlepass::new()).engine();
                let store = Store::new(&engine);
                self.0.insert(
                    key,
                    Module::new(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?,
                );
                self.get(key, wasm)
            }
        }
    }
}
