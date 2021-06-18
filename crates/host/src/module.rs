use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use wasmer::Cranelift;
use wasmer::Module;
use wasmer::Store;
use wasmer::Universal;

#[derive(Default)]
pub struct SerializedModuleCache(HashMap<[u8; 32], Vec<u8>>);

pub static SERIALIZED_MODULE_CACHE: Lazy<RwLock<SerializedModuleCache>> = Lazy::new(|| RwLock::new(SerializedModuleCache::default()));

impl SerializedModuleCache {
    fn get_with_build_cache(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Module, WasmError> {
        let store = Store::new(&Universal::new(Cranelift::default()).engine());
        let module =
            Module::from_binary(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
        let serialized_module = module
            .serialize()
            .map_err(|e| WasmError::Compile(e.to_string()))?;
        self.0.insert(key, serialized_module);
        Ok(module)
    }

    pub fn get(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Module, WasmError> {
        match self.0.get(&key) {
            Some(serialized_module) => {
                let store = Store::new(&Universal::new(Cranelift::default()).engine());
                let module = unsafe { Module::deserialize(&store, serialized_module) }
                .map_err(|e| WasmError::Compile(e.to_string()))?;
                Ok(module)
            },
            None => {
                self.get_with_build_cache(key, wasm)
            },
        }
    }
}

#[derive(Default)]
pub struct ModuleCache(HashMap<[u8; 32], Arc<Module>>);

pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> =
    Lazy::new(|| RwLock::new(ModuleCache::default()));

impl ModuleCache {
    fn get_with_build_cache(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        let module = SERIALIZED_MODULE_CACHE.write().get(key, wasm)?;
        let arc = Arc::new(module);
        self.0.insert(key, Arc::clone(&arc));
        Ok(arc)
    }

    pub fn get(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        match self.0.get(&key) {
            Some(module) => Ok(Arc::clone(module)),
            None => self.get_with_build_cache(key, wasm),
        }
    }
}