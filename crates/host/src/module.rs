use crate::plru::MicroCache;
use crate::prelude::*;
use bimap::BiMap;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use holochain_wasmer_common::WasmError;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use wasmer::BaseTunables;
use wasmer::Cranelift;
use wasmer::Engine;
use wasmer::Instance;
use wasmer::Module;
use wasmer::NativeEngineExt;
use wasmer::Store;

/// We expect cache keys to be produced via hashing so 32 bytes is enough for all
/// purposes.
pub type CacheKey = [u8; 32];
/// Plru uses a usize to track "recently used" so we need a map between 32 byte cache
/// keys and the bits used to evict things from the cache.
pub type PlruKeyMap = BiMap<usize, CacheKey>;
/// Modules serialize to a vec of bytes as per wasmer.
pub type SerializedModule = Bytes;

#[derive(Clone, Debug)]
pub struct ModuleWithStore {
    pub store: Arc<Mutex<Store>>,
    pub module: Arc<Module>,
}

#[derive(Clone, Debug)]
pub struct InstanceWithStore {
    pub store: Arc<Mutex<Store>>,
    pub instance: Arc<Instance>,
}

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
#[derive(Debug)]
pub struct SerializedModuleCache {
    pub plru: MicroCache,
    pub key_map: PlruKeyMap,
    pub cache: BTreeMap<CacheKey, Arc<SerializedModule>>,
    pub cranelift: fn() -> Cranelift,
    pub maybe_fs_dir: Option<PathBuf>,
}

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
            maybe_fs_dir: None,
        }
    }

    fn module_path(&self, key: CacheKey) -> Option<PathBuf> {
        self.maybe_fs_dir
            .as_ref()
            .map(|dir_path| dir_path.clone().join(hex::encode(key)))
    }

    fn store(&self) -> Store {
        let mut engine: Engine = (self.cranelift)().into();
        // Workaround for invalid memory access on iOS.
        // https://github.com/holochain/holochain/issues/3096
        engine.set_tunables(BaseTunables {
            static_memory_bound: 0x4000.into(),
            static_memory_offset_guard_size: 0x1_0000,
            dynamic_memory_offset_guard_size: 0x1_0000,
        });
        Store::new(engine)
    }

    /// Given a wasm, compiles with cranelift, serializes the result, adds it to
    /// the cache and returns that.
    fn get_with_build_cache(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<ModuleWithStore>, wasmer::RuntimeError> {
        let store = self.store();

        let maybe_module_path = self.module_path(key);
        let (module, serialized_module) = match maybe_module_path.as_ref().map(|module_path| {
            // We do this the long way to get `Bytes` instead of `Vec<u8>` so
            // that the clone when we both deserialize and cache is cheap.
            let mut file = File::open(module_path).map_err(|e| {
                wasm_error!(WasmErrorInner::Compile(format!(
                    "{} Path: {}",
                    e,
                    module_path.display()
                )))
            })?;
            let mut bytes_mut = BytesMut::new().writer();

            std::io::copy(&mut file, &mut bytes_mut).map_err(|e| {
                wasm_error!(WasmErrorInner::Compile(format!(
                    "{} Path: {}",
                    e,
                    module_path.display()
                )))
            })?;
            Ok::<bytes::Bytes, wasmer::RuntimeError>(bytes_mut.into_inner().freeze())
        }) {
            Some(Ok(serialized_module)) => (
                unsafe { Module::deserialize(&store, serialized_module.clone()) }
                    .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?,
                serialized_module,
            ),
            _fs_miss => {
                let module = Module::from_binary(&store, wasm)
                    .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;
                let serialized_module = module
                    .serialize()
                    .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;

                if let Some(module_path) = maybe_module_path {
                    match OpenOptions::new()
                        .write(true)
                        // Using create_new here so that cache stampedes don't
                        // cause corruption. Each file can only be written once.
                        .create_new(true)
                        .open(&module_path)
                    {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(&serialized_module) {
                                tracing::error!("{} Path: {}", e, module_path.display());
                            }
                        }
                        Err(e) => {
                            // This is just a warning because it is expected that
                            // multiple concurrent calls to build the same wasm
                            // will sometimes happen.
                            tracing::warn!("{} Path: {}", e, module_path.display());
                        }
                    }
                }

                (module, serialized_module)
            }
        };
        self.put_item(key, Arc::new(serialized_module.clone()));

        Ok(Arc::new(ModuleWithStore {
            store: Arc::new(Mutex::new(store)),
            module: Arc::new(module),
        }))
    }

    /// Given a wasm, attempts to get the serialized module for it from the cache.
    /// If the cache misses a new serialized module, will be built from the wasm.
    pub fn get(
        &mut self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<ModuleWithStore>, wasmer::RuntimeError> {
        match self.cache.get(&key) {
            Some(serialized_module) => {
                let store = self.store();
                let module = unsafe { Module::deserialize(&store, (**serialized_module).clone()) }
                    .map_err(|e| wasm_error!(WasmErrorInner::Compile(e.to_string())))?;
                self.touch(&key);
                Ok(Arc::new(ModuleWithStore {
                    store: Arc::new(Mutex::new(store)),
                    module: Arc::new(module),
                }))
            }
            None => self.get_with_build_cache(key, wasm),
        }
    }
}

/// Caches wasm instances. Reusing wasm instances allows maximum speed in function
/// calls but also introduces the possibility of memory corruption or other bad
/// state that is inappropriate to persist/reuse/access across calls. It is the
/// responsibility of the host to discard instances that are not eligible for reuse.
#[derive(Default, Debug)]
pub struct InstanceCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: BTreeMap<CacheKey, Arc<InstanceWithStore>>,
}

impl PlruCache for InstanceCache {
    type Item = InstanceWithStore;

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
