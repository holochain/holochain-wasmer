//! Wasmer Host Module Manager
//!
//! This provides two ways to build & access wasm modules:
//!
//! 1. When using the feature flag `wasmer_sys`, modules should be accessed only via the [`ModuleCache`].
//!    This ensures that wasm modules are compiled once, then cached and stored efficiently.
//!
//! 2. When using the feature flag `wasmer_wamr`, modules should be built via the exported build_module function.
//!    There is no need for caching, as the wasm module is interpreted.

use crate::plru::MicroCache;
use crate::prelude::*;
use bimap::BiMap;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;

mod builder;
pub use builder::ModuleBuilder;

#[cfg(feature = "wasmer_sys")]
mod wasmer_sys;
#[cfg(feature = "wasmer_sys")]
pub use wasmer_sys::*;

#[cfg(feature = "wasmer_wamr")]
mod wasmer_wamr;
#[cfg(feature = "wasmer_wamr")]
pub use wasmer_wamr::*;

/// We expect cache keys to be produced via hashing so 32 bytes is enough for all
/// purposes.
pub type CacheKey = [u8; 32];
/// Plru uses a usize to track "recently used" so we need a map between 32 byte cache
/// keys and the bits used to evict things from the cache.
type PlruKeyMap = BiMap<usize, CacheKey>;

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
trait PlruCache {
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

    /// Attempt to retrieve an item from the cache by its cache key.
    fn get_item(&mut self, key: &CacheKey) -> Option<Arc<Self::Item>> {
        let maybe_item = self.cache().get(key).cloned();
        if maybe_item.is_some() {
            self.touch(key);
        }
        maybe_item
    }
}

/// Caches deserialized wasm modules.
///
/// Deserialization of cached modules from the cache to create callable instances is slow.
/// Therefore modules are cached in memory after deserialization.
#[derive(Default, Debug)]
struct InMemoryModuleCache {
    plru: MicroCache,
    key_map: PlruKeyMap,
    cache: BTreeMap<CacheKey, Arc<Module>>,
}

impl PlruCache for InMemoryModuleCache {
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

    fn cache(&self) -> &BTreeMap<CacheKey, Arc<Self::Item>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut BTreeMap<CacheKey, Arc<Self::Item>> {
        &mut self.cache
    }
}

#[derive(Debug)]
pub struct ModuleCache {
    // The in-memory cache of deserialized modules
    cache: Arc<RwLock<InMemoryModuleCache>>,

    // Filesystem path where serialized modules are cached.
    //
    // A serialized wasm module must still be deserialized before it can be used to build instances.
    // The deserialization process is far faster than compiling and much slower than instance building.
    filesystem_path: Option<PathBuf>,

    // Handles building wasm modules
    //
    // It includes the runtime engine that must live as long as the module,
    // so we keep it in the cache and use for all modules.
    builder: ModuleBuilder,
}

impl ModuleCache {
    pub fn new(filesystem_path: Option<PathBuf>) -> Self {
        let cache = Arc::new(RwLock::new(InMemoryModuleCache::default()));
        ModuleCache {
            cache,
            filesystem_path,
            builder: ModuleBuilder::new(),
        }
    }

    /// Get a module from the cache, or add it to both caches if not found
    pub fn get(&self, key: CacheKey, wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
        // Check in-memory cache for module
        if let Some(module) = self.get_from_cache(key) {
            return Ok(module);
        }

        // Check the filesystem for module
        match self.get_from_filesystem(key) {
            // Filesystem cache hit, deserialize and save to cache
            Ok(Some(serialized_module)) => {
                let module_result = self.builder.from_serialized_module(serialized_module);

                // If deserialization fails, we assume the file is corrupt,
                // so it is removed from the filesystem cache,
                // and the wasm is re-added to the cache again.
                let module = match module_result {
                    Ok(d) => Ok(d),
                    Err(_) => {
                        // Remove from filesystem
                        if let Err(e) = self.remove_from_filesystem(key) {
                            tracing::debug!("Failed to remove cached wasm from filesystem with cache key {:?}: {:?}", key, e);
                        }

                        // Build wasm and save to both caches
                        self.add_to_cache_and_filesystem(key, wasm)
                    }
                }?;

                self.add_to_cache(key, module.clone());

                Ok(module)
            }

            // Filesystem cache miss, build wasm and save to both caches
            _ => self.add_to_cache_and_filesystem(key, wasm),
        }
    }

    /// Build a wasm, then save it to both the in-memory cache and filesystem
    fn add_to_cache_and_filesystem(
        &self,
        key: CacheKey,
        wasm: &[u8],
    ) -> Result<Arc<Module>, wasmer::RuntimeError> {
        // Each module needs to be compiled with a new engine because
        // of middleware like metering. Middleware is compiled into the
        // module once and available in all instances created from it.
        let module = self.builder.from_binary(wasm)?;

        // Round trip the wasmer Module through serialization.
        //
        // A new middleware per module is required, hence a new engine
        // per module is needed too. Serialization allows for uncoupling
        // the module from the engine that was used for compilation.
        // After that another engine can be used to deserialize the
        // module again. The engine has to live as long as the module to
        // prevent memory access out of bounds errors.
        //
        // This procedure facilitates caching of modules that can be
        // instantiated with fresh stores free from state. Instance
        // creation is highly performant which makes caching of instances
        // and stores unnecessary.
        //
        // See https://github.com/wasmerio/wasmer/discussions/3829#discussioncomment-5790763
        let serialized_module = module
            .serialize()
            .map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?;
        let module = self
            .builder
            .from_serialized_module(serialized_module.clone())?;

        // Save serialized module to filesystem cache
        self.add_to_filesystem(key, serialized_module)?;

        // Save module to in-memory cache
        self.add_to_cache(key, module.clone());

        Ok(module)
    }

    /// Get serialized module from filesystem
    fn get_from_filesystem(&self, key: CacheKey) -> Result<Option<Bytes>, wasmer::RuntimeError> {
        self.filesystem_module_path(key)
            .as_ref()
            .map(|module_path| {
                // Read file into `Bytes` instead of `Vec<u8>` so that the clone is cheap
                let mut file = File::open(module_path).map_err(|e| {
                    wasm_error!(WasmErrorInner::ModuleBuild(format!(
                        "{} Path: {}",
                        e,
                        module_path.display()
                    )))
                })?;

                let mut bytes_mut = BytesMut::new().writer();
                std::io::copy(&mut file, &mut bytes_mut).map_err(|e| {
                    wasm_error!(WasmErrorInner::ModuleBuild(format!(
                        "{} Path: {}",
                        e,
                        module_path.display()
                    )))
                })?;

                Ok::<bytes::Bytes, wasmer::RuntimeError>(bytes_mut.into_inner().freeze())
            })
            .transpose()
    }

    /// Add serialized module to filesystem cache
    fn add_to_filesystem(
        &self,
        key: CacheKey,
        serialized_module: Bytes,
    ) -> Result<(), wasmer::RuntimeError> {
        if let Some(fs_path) = self.filesystem_module_path(key) {
            match OpenOptions::new()
                .write(true)
                // Using create_new here so that cache stampedes don't
                // cause corruption. Each file can only be written once.
                .create_new(true)
                .open(&fs_path)
            {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(&serialized_module) {
                        tracing::error!("{} Path: {}", e, fs_path.display());
                    }
                }
                Err(e) => {
                    // This is just a warning because it is expected that
                    // multiple concurrent calls to build the same wasm
                    // will sometimes happen.
                    tracing::warn!("{} Path: {}", e, fs_path.display());
                }
            }
        }

        Ok(())
    }

    // Remove serialized module from filesystem cache
    fn remove_from_filesystem(&self, key: CacheKey) -> Result<(), std::io::Error> {
        if let Some(fs_path) = self.filesystem_module_path(key) {
            std::fs::remove_file(fs_path)?;
        }

        Ok(())
    }

    /// Check cache for module
    fn get_from_cache(&self, key: CacheKey) -> Option<Arc<Module>> {
        let mut cache = self.cache.write();
        cache.get_item(&key)
    }

    /// Add module to cache
    fn add_to_cache(&self, key: CacheKey, module: Arc<Module>) {
        let mut cache = self.cache.write();
        cache.put_item(key, module.clone());
    }

    /// Get filesystem cache path for a given key
    fn filesystem_module_path(&self, key: CacheKey) -> Option<PathBuf> {
        self.filesystem_path
            .as_ref()
            .map(|dir_path| dir_path.clone().join(hex::encode(key)))
    }
}
