use crate::plru::MicroCache;
use crate::prelude::*;
use bimap::BiMap;
use holochain_wasmer_common::WasmError;
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use wasmer::Cranelift;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
// use wasmer::Universal;
use bytes::Bytes;

/// We expect cache keys to be produced via hashing so 32 bytes is enough for all
/// purposes.
pub type CacheKey = [u8; 32];
/// Plru uses a usize to track "recently used" so we need a map between 32 byte cache
/// keys and the bits used to evict things from the cache.
pub type PlruKeyMap = BiMap<usize, CacheKey>;
/// Modules serialize to a vec of bytes as per wasmer.
pub type SerializedModule = Bytes;

/// Higher level trait over the plru cache to make it a bit easier to interact
/// with consistently. Default implementations for key functions are provided.
/// Notably handles keeping the mapping between cache keys and items, and the
/// plru tracking including touching and evicting.
pub trait PlruCache {
    /// The type of items in the cache.
    type Item;
    /// Accessor for mutable reference to internal plru cache.
    fn plru_mut(&mut self) -> &mut MicroCache;
    /// Accessor to mapping between plru cache bits and cache keys.
    fn key_map(&self) -> &PlruKeyMap;
    /// Mutable accessor to mapping between plru cache bits and cache keys.
    fn key_map_mut(&mut self) -> &mut PlruKeyMap;
    /// Accessor to the cache key addressable cache of items.
    fn cache(&self) -> &BTreeMap<CacheKey, Arc<Self::Item>>;
    /// Mutable accessor to the cache key addressable cache of items.
    fn cache_mut(&mut self) -> &mut BTreeMap<CacheKey, Arc<Self::Item>>;
    /// Put an item in both the plru cache and the item cache by its cache key.
    /// If the cache is full, the roughly most stale plru will be evicted from the
    /// item cache and reassigned to the new item.
    fn put_item(&mut self, key: CacheKey, item: Arc<Self::Item>) -> Arc<Self::Item> {
        let plru_key = self.plru_mut().replace();
        // If there is something in the cache for this plru slot already drop it.
        if let Some(stale_key) = self.key_map().get_by_left(&plru_key).cloned() {
            self.cache_mut().remove(&stale_key);
        }
        self.cache_mut().insert(key, Arc::clone(&item));
        self.plru_mut().touch(plru_key);
        self.key_map_mut().insert(plru_key, key);
        item
    }
    /// Get the plru key for a given cache key. Will panic if the mapping does not
    /// exist so the caller MUST NOT request the plru on a cache miss AND that the
    /// cache is never set such as to cause a hit without also setting the plru.
    fn plru_key(&self, key: &CacheKey) -> usize {
        *self
            .key_map()
            .get_by_right(key)
            // It is a bug to get a plru key on a cache MISS.
            // It is also a bug if a cache HIT does not map to a plru key.
            .expect("Missing cache plru key mapping. This is a bug.")
    }

    /// Touches the plru such that the given CacheKey becomes the most recently
    /// used item.
    fn touch(&mut self, key: &CacheKey) {
        let plru_key = self.plru_key(key);
        self.plru_mut().touch(plru_key);
    }

    /// Delete the plru for a given cache key. Care must be taken to ensure this
    /// is not called before a subsequent call to `plru_key` or it will panic.
    fn trash(&mut self, key: &CacheKey) {
        let plru_key = self.plru_key(key);
        self.plru_mut().trash(plru_key);
    }

    /// Remove an item from the cache and the associated plru entry.
    fn remove_item(&mut self, key: &CacheKey) -> Option<Arc<Self::Item>> {
        let maybe_item = self.cache_mut().remove(key);
        if maybe_item.is_some() {
            let plru_key = self.plru_key(key);
            self.plru_mut().trash(plru_key);
            self.key_map_mut().remove_by_left(&plru_key);
        }
        maybe_item
    }

    /// Attempt to retrieve an item from the cache by its cache key.
    fn get_item(&mut self, key: &CacheKey) -> Option<Arc<Self::Item>> {
        let maybe_item = self.cache().get(key).cloned();
        if maybe_item.is_some() {
            self.touch(key);
        }
        maybe_item
    }
}

/// Cache for serialized modules. These are fully compiled wasm modules that are
/// then serialized by wasmer and can be cached. A serialized wasm module must still
/// be deserialized before it can be used to build instances. The deserialization
/// process is far faster than compiling and much slower than instance building.
pub struct SerializedModuleCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: BTreeMap<CacheKey, Arc<SerializedModule>>,
    cranelift: fn() -> Cranelift,
}

pub static SERIALIZED_MODULE_CACHE: OnceCell<RwLock<SerializedModuleCache>> = OnceCell::new();

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

    fn cache(&self) -> &BTreeMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut BTreeMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}

impl SerializedModuleCache {
    /// Build a default `SerializedModuleCache` with a `Cranelift` that will be used
    /// to compile modules for serialization as needed.
    pub fn default_with_cranelift(cranelift: fn() -> Cranelift) -> Self {
        Self {
            cranelift,
            plru: MicroCache::default(),
            key_map: PlruKeyMap::default(),
            cache: BTreeMap::default(),
        }
    }

    /// Given a wasm, compiles with cranelift, serializes the result, adds it to
    /// the cache and returns that.
    fn get_with_build_cache(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<(Store, Module), wasmer::RuntimeError> {
        let store = Store::new((self.cranelift)());
        let module = Module::from_binary(&store, wasm)
            .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;
        let serialized_module = module
            .serialize()
            .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;
        self.put_item(key, Arc::new(serialized_module));
        Ok((store, module))
    }

    /// Given a wasm, attempts to get the serialized module for it from the cache.
    /// If the cache misses a new serialized module, will be built from the wasm.
    pub fn get(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<(Store, Module), wasmer::RuntimeError> {
        match self.cache.get(&key) {
            Some(serialized_module) => {
                let store = Store::new((self.cranelift)());
                let module = unsafe { Module::deserialize(&store, (**serialized_module).clone()) }
                    .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;
                self.touch(&key);
                Ok((store, module))
            }
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

/// Caches wasmer modules that can be used to build wasmer instances. This is the
/// output of building from wasm or deserializing the items in the serialized module cache.
#[derive(Default)]
pub struct ModuleCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: BTreeMap<CacheKey, Arc<(Mutex<Store>, Module)>>,
}

pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> =
    Lazy::new(|| RwLock::new(ModuleCache::default()));

impl ModuleCache {
    /// Wraps the serialized module cache to build modules as needed and also cache
    /// the module itself in the module cache.
    fn get_with_build_cache(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<(Mutex<Store>, Module)>, wasmer::RuntimeError> {
        let (store, module) = match SERIALIZED_MODULE_CACHE.get() {
            Some(serialized_module_cache) => serialized_module_cache.write().get(key, wasm)?,
            None => {
                return Err(wasmer::RuntimeError::user(Box::new(wasm_error!(
                    WasmErrorInner::UninitializedSerializedModuleCache
                ))))
            }
        };
        Ok(self.put_item(key, Arc::new((Mutex::new(store), module))))
    }

    /// Attempts to retrieve a module ready to build instances from. Builds a new
    /// module from the provided wasm and caches both the module and a serialized
    /// copy of the module if there is a miss.
    pub fn get(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<(Mutex<Store>, Module)>, wasmer::RuntimeError> {
        match self.get_item(&key) {
            Some(item) => Ok(item),
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

impl PlruCache for ModuleCache {
    type Item = (Mutex<Store>, Module);

    fn plru_mut(&mut self) -> &mut MicroCache {
        &mut self.plru
    }

    fn key_map_mut(&mut self) -> &mut PlruKeyMap {
        &mut self.key_map
    }

    fn key_map(&self) -> &PlruKeyMap {
        &self.key_map
    }

    fn cache(&self) -> &BTreeMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut BTreeMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}

/// Caches wasm instances. Reusing wasm instances allows maximum speed in function
/// calls but also introduces the possibility of memory corruption or other bad
/// state that is inappropriate to persist/reuse/access across calls. It is the
/// responsibility of the host to discard instances that are not eligible for reuse.
#[derive(Default)]
pub struct InstanceCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: BTreeMap<CacheKey, Arc<(Mutex<Store>, Mutex<Instance>)>>,
}
pub static INSTANCE_CACHE: Lazy<RwLock<InstanceCache>> =
    Lazy::new(|| RwLock::new(InstanceCache::default()));

impl PlruCache for InstanceCache {
    type Item = (Mutex<Store>, Mutex<Instance>);

    fn plru_mut(&mut self) -> &mut MicroCache {
        &mut self.plru
    }

    fn key_map_mut(&mut self) -> &mut PlruKeyMap {
        &mut self.key_map
    }

    fn key_map(&self) -> &PlruKeyMap {
        &self.key_map
    }

    fn cache(&self) -> &BTreeMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut BTreeMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}
