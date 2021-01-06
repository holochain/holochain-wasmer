use lazy_static::lazy_static;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use wasmer_runtime::cache::Cache;
use wasmer_runtime::cache::FileSystemCache;
use wasmer_runtime::cache::WasmHash;
use wasmer_runtime::error::CacheError;
use wasmer_runtime::Backend;
use wasmer_runtime::Module;

use std::sync::Mutex;

lazy_static! {
    static ref HASHCACHE: Mutex<HashMap<String, HashMap<WasmHash, Module>>> =
        Mutex::new(HashMap::new());
}

pub struct MemoryFallbackFileSystemCache {
    fs_fallback: Option<FileSystemCache>,
}

impl MemoryFallbackFileSystemCache {
    pub fn new<P: Into<PathBuf>>(
        maybe_fallback_path: Option<P>,
    ) -> io::Result<MemoryFallbackFileSystemCache> {
        Ok(MemoryFallbackFileSystemCache {
            fs_fallback: match maybe_fallback_path {
                Some(fallback_path) => Some(unsafe { FileSystemCache::new(fallback_path) }?),
                None => None,
            },
        })
    }

    fn load_with_backend_mem(&self, key: WasmHash, backend: Backend) -> Result<Module, CacheError> {
        let cache = HASHCACHE.lock().unwrap();
        let backend_key = backend.to_string();
        match cache.get(backend_key) {
            Some(module_cache) => match module_cache.get(&key) {
                Some(module) => Ok(module.to_owned()),
                _ => Err(CacheError::InvalidatedCache),
            },
            _ => Err(CacheError::InvalidatedCache),
        }
    }

    fn load_with_backend_fs(&self, key: WasmHash, backend: Backend) -> Result<Module, CacheError> {
        // we did not find anything in memory so fallback to fs
        if let Some(fs_fallback) = &self.fs_fallback {
            match fs_fallback.load_with_backend(key, backend) {
                Ok(module) => {
                    // update the memory cache so we load faster next time
                    self.store_mem(key, module.clone())?;
                    Ok(module)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(CacheError::InvalidatedCache)
        }
    }

    fn store_mem(&self, key: WasmHash, module: Module) -> Result<(), CacheError> {
        let mut cache = HASHCACHE.lock().unwrap();
        let backend_key = module.info().backend.to_string();
        let backend_map = cache.entry(backend_key).or_insert_with(HashMap::new);
        backend_map.entry(key).or_insert(module);
        Ok(())
    }

    fn store_fs(&mut self, key: WasmHash, module: Module) -> Result<(), CacheError> {
        if let Some(fs_fallback) = &mut self.fs_fallback {
            fs_fallback.store(key, module)?;
        }
        Ok(())
    }
}

impl Cache for MemoryFallbackFileSystemCache {
    type LoadError = CacheError;
    type StoreError = CacheError;

    fn load(&self, key: WasmHash) -> Result<Module, CacheError> {
        self.load_with_backend(key, Backend::default())
    }

    fn load_with_backend(&self, key: WasmHash, backend: Backend) -> Result<Module, CacheError> {
        match self.load_with_backend_mem(key, backend) {
            Ok(module) => Ok(module),
            Err(_) => self.load_with_backend_fs(key, backend),
        }
    }

    fn store(&mut self, key: WasmHash, module: Module) -> Result<(), CacheError> {
        // store in depth first order
        self.store_fs(key, module.clone())?;
        self.store_mem(key, module)?;

        Ok(())
    }
}
