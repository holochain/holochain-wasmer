// use crate::prelude::Instance;
use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
// use parking_lot::Mutex;
use parking_lot::RwLock;
use plru::MicroCache;
use std::collections::HashMap;
// use std::sync::atomic::AtomicU64;
use bimap::BiMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use wasmer::Cranelift;
use wasmer::Module;
use wasmer::Store;
use wasmer::Universal;

pub type ModuleCacheKey = [u8; 32];
pub type SerializedModule = Vec<u8>;

#[derive(Default)]
pub struct SerializedModuleCache {
    plru: MicroCache,
    key_map: BiMap<usize, ModuleCacheKey>,
    cache: HashMap<ModuleCacheKey, SerializedModule>,
}

pub static SERIALIZED_MODULE_CACHE: Lazy<RwLock<SerializedModuleCache>> =
    Lazy::new(|| RwLock::new(SerializedModuleCache::default()));

impl SerializedModuleCache {
    fn put(&mut self, key: ModuleCacheKey, serialized_module: SerializedModule) {
        let plru_key = self.plru.replace();
        // if there is something in the cache in this plru slot already drop it.
        if let Some(stale_key) = self.key_map.get_by_left(&plru_key) {
            self.cache.remove(stale_key);
        }
        self.cache.insert(key, serialized_module);
        self.plru.touch(plru_key);
        self.key_map.insert(plru_key, key);
    }

    fn touch(&mut self, key: ModuleCacheKey) {
        self.plru.touch(
            *self
                .key_map
                .get_by_right(&key)
                // It is a bug to call touch on a cache MISS.
                // It is also a bug if a cache HIT does not map to a plru key.
                .expect("Missing serialized cache plru key mapping. This is a bug."),
        )
    }

    fn get_with_build_cache(
        &mut self,
        key: ModuleCacheKey,
        wasm: &[u8],
    ) -> Result<Module, WasmError> {
        let store = Store::new(&Universal::new(Cranelift::default()).engine());
        let module =
            Module::from_binary(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
        let serialized_module = module
            .serialize()
            .map_err(|e| WasmError::Compile(e.to_string()))?;
        self.put(key, serialized_module);
        Ok(module)
    }

    pub fn get(&mut self, key: ModuleCacheKey, wasm: &[u8]) -> Result<Module, WasmError> {
        match self.cache.get(&key) {
            Some(serialized_module) => {
                let store = Store::new(&Universal::new(Cranelift::default()).engine());
                let module = unsafe { Module::deserialize(&store, serialized_module) }
                    .map_err(|e| WasmError::Compile(e.to_string()))?;
                self.touch(key);
                Ok(module)
            }
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

#[derive(Default)]
pub struct ModuleCache(HashMap<ModuleCacheKey, Arc<Module>>, bool, AtomicUsize);

pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> =
    Lazy::new(|| RwLock::new(ModuleCache::default()));

impl ModuleCache {
    fn get_with_build_cache(
        &mut self,
        key: ModuleCacheKey,
        wasm: &[u8],
    ) -> Result<Arc<Module>, WasmError> {
        let module = SERIALIZED_MODULE_CACHE.write().get(key, wasm)?;
        let arc = Arc::new(module);
        self.0.insert(key, Arc::clone(&arc));
        Ok(arc)
    }

    pub fn reset_counter(&mut self) {
        self.2.store(0, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn get(&mut self, key: ModuleCacheKey, wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        let count = self.2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count > 100 {
            let current = self.0.remove(&key);
            match current {
                Some(current) => match Arc::try_unwrap(current) {
                    Ok(m) => {
                        std::mem::drop(m);
                        self.2.store(0, std::sync::atomic::Ordering::SeqCst);
                        self.get_with_build_cache(key, wasm)
                    }
                    Err(current) => {
                        self.0.insert(key, current.clone());
                        Ok(current)
                    }
                },
                None => {
                    self.2.store(0, std::sync::atomic::Ordering::SeqCst);
                    self.get_with_build_cache(key, wasm)
                }
            }
        } else {
            match self.0.get(&key) {
                Some(module) => Ok(Arc::clone(module)),
                None => self.get_with_build_cache(key, wasm),
            }
        }
    }
}

// #[derive(Default)]
// pub struct InstanceCache(<Arc<Mutex<HashMap<ModuleCacheKey, (HashMap<u64, Arc<Mutex<Instance>>>, AtomicU64)>>>>;
// static INSTANCE_CACHE: Lazy = Lazy::new(Default::default);

// const INSTANCE_CACHE_SIZE: usize = 20;
