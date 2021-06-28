use crate::prelude::Instance;
use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
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

pub type CacheKey = [u8; 32];
pub type PlruKeyMap = BiMap<usize, CacheKey>;
pub type SerializedModule = Vec<u8>;

pub trait PlruCache {
    type Item;

    fn plru_mut(&mut self) -> &mut MicroCache;
    fn key_map(&self) -> &PlruKeyMap;
    fn key_map_mut(&mut self) -> &mut PlruKeyMap;
    fn cache(&self) -> &HashMap<CacheKey, Arc<Self::Item>>;
    fn cache_mut(&mut self) -> &mut HashMap<CacheKey, Arc<Self::Item>>;

    fn put_item(&mut self, key: CacheKey, item: Arc<Self::Item>) -> Arc<Self::Item> {
        let plru_key = self.plru_mut().replace();
        // if there is something in the cache for this plru slot already drop it.
        if let Some(stale_key) = self.key_map().get_by_left(&plru_key).cloned() {
            self.cache_mut().remove(&stale_key);
        }
        self.cache_mut().insert(key, Arc::clone(&item));
        self.plru_mut().touch(plru_key);
        self.key_map_mut().insert(plru_key, key);
        item
    }

    fn plru_key(&self, key: &CacheKey) -> usize {
        *self
            .key_map()
            .get_by_right(key)
            // It is a bug to get a plru key on a cache MISS.
            // It is also a bug if a cache HIT does not map to a plru key.
            .expect("Missing cache plru key mapping. This is a bug.")
    }

    fn touch(&mut self, key: &CacheKey) {
        let plru_key = self.plru_key(key);
        self.plru_mut().touch(plru_key);
    }

    fn trash(&mut self, key: &CacheKey) {
        let plru_key = self.plru_key(key);
        self.plru_mut().trash(plru_key);
    }

    fn remove_item(&mut self, key: &CacheKey) -> Option<Arc<Self::Item>> {
        let maybe_item = self.cache_mut().remove(key);
        if maybe_item.is_some() {
            let plru_key = self.plru_key(key);
            self.plru_mut().trash(plru_key);
            self.key_map_mut().remove_by_left(&plru_key);
        }
        maybe_item
    }

    fn get_item(&mut self, key: &CacheKey) -> Option<Arc<Self::Item>> {
        let maybe_item = self.cache().get(key).cloned();
        if maybe_item.is_some() {
            self.touch(key);
        }
        maybe_item
    }
}

#[derive(Default)]
pub struct SerializedModuleCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: HashMap<CacheKey, Arc<SerializedModule>>,
}

pub static SERIALIZED_MODULE_CACHE: Lazy<RwLock<SerializedModuleCache>> =
    Lazy::new(|| RwLock::new(SerializedModuleCache::default()));

impl PlruCache for SerializedModuleCache {
    type Item = SerializedModule;

    fn plru_mut(&mut self) -> &mut MicroCache {
        &mut self.plru
    }

    fn key_map_mut(&mut self) -> &mut PlruKeyMap {
        &mut self.key_map
    }

    fn key_map(&self) -> &PlruKeyMap {
        &self.key_map
    }

    fn cache(&self) -> &HashMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut HashMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}

impl SerializedModuleCache {
    fn get_with_build_cache(&mut self, key: CacheKey, wasm: &[u8]) -> Result<Module, WasmError> {
        let store = Store::new(&Universal::new(Cranelift::default()).engine());
        let module =
            Module::from_binary(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
        let serialized_module = module
            .serialize()
            .map_err(|e| WasmError::Compile(e.to_string()))?;
        self.put_item(key, Arc::new(serialized_module));
        Ok(module)
    }

    pub fn get(&mut self, key: CacheKey, wasm: &[u8]) -> Result<Module, WasmError> {
        match self.cache.get(&key) {
            Some(serialized_module) => {
                let store = Store::new(&Universal::new(Cranelift::default()).engine());
                let module = unsafe { Module::deserialize(&store, serialized_module) }
                    .map_err(|e| WasmError::Compile(e.to_string()))?;
                self.touch(&key);
                Ok(module)
            }
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

#[derive(Default)]
pub struct ModuleCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: HashMap<CacheKey, Arc<Module>>,
    leak_buster: AtomicUsize,
}

pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> =
    Lazy::new(|| RwLock::new(ModuleCache::default()));

impl ModuleCache {
    pub fn reset_leak_buster(&mut self) {
        self.leak_buster
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn should_bust_leak(&mut self) -> bool {
        if self
            .leak_buster
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            > 100
        {
            self.reset_leak_buster();
            true
        } else {
            false
        }
    }

    fn get_with_build_cache(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<Module>, WasmError> {
        let module = SERIALIZED_MODULE_CACHE.write().get(key, wasm)?;
        Ok(self.put_item(key, Arc::new(module)))
    }

    pub fn get(&mut self, key: CacheKey, wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        match if self.should_bust_leak() {
            self.remove_item(&key)
        } else {
            self.get_item(&key)
        } {
            Some(module) => Ok(module),
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

impl PlruCache for ModuleCache {
    type Item = Module;

    fn plru_mut(&mut self) -> &mut MicroCache {
        &mut self.plru
    }

    fn key_map_mut(&mut self) -> &mut PlruKeyMap {
        &mut self.key_map
    }

    fn key_map(&self) -> &PlruKeyMap {
        &self.key_map
    }

    fn cache(&self) -> &HashMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut HashMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}

#[derive(Default)]
pub struct InstanceCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: HashMap<CacheKey, Arc<Mutex<Instance>>>,
}
pub static INSTANCE_CACHE: Lazy<RwLock<InstanceCache>> =
    Lazy::new(|| RwLock::new(InstanceCache::default()));

impl PlruCache for InstanceCache {
    type Item = Mutex<Instance>;

    fn plru_mut(&mut self) -> &mut MicroCache {
        &mut self.plru
    }

    fn key_map_mut(&mut self) -> &mut PlruKeyMap {
        &mut self.key_map
    }

    fn key_map(&self) -> &PlruKeyMap {
        &self.key_map
    }

    fn cache(&self) -> &HashMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut HashMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}
