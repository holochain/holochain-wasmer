use crate::prelude::*;
use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
use wasmer::JIT;
use wasmer_compiler_singlepass::Singlepass;
use rand::prelude::*;

pub struct ModuleCache(HashMap<[u8; 32], Arc<Module>>);
pub struct InstanceCache(HashMap<[u8; 32], Arc<Mutex<Instance>>>);

pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> = Lazy::new(|| RwLock::new(ModuleCache::new()));

impl ModuleCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    fn get_with_build_cache(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        let store = Store::new(&JIT::new(Singlepass::new()).engine());
        let module = Module::from_binary(&store, wasm).map_err(|e| WasmError::Compile(e.to_string()))?;
        self.0.insert(
            key,
            Arc::new(module)
        );
        self.get(key, wasm)
    }

    pub fn get(
        &mut self,
        key: [u8; 32],
        wasm: &[u8]
    ) -> Result<Arc<Module>, WasmError> {
        let mut rng = rand::thread_rng();
        if rng.gen::<u8>() == 0 {
            let _ = self.0.remove(&key);
            self.get(key, wasm)
        } else {
            match self.0.get(&key) {
                Some(module) => Ok(Arc::clone(module)),
                None => self.get_with_build_cache(key, wasm),
            }
        }
    }
}

pub static INSTANCE_CACHE: Lazy<RwLock<InstanceCache>> =
    Lazy::new(|| RwLock::new(InstanceCache::new()));

impl InstanceCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(
        &mut self,
        key: [u8; 32],
        wasm: &[u8],
        import_builder: fn(&Store, &Env) -> ImportObject,
    ) -> Result<Arc<Mutex<Instance>>, WasmError> {
        match self.0.get(&key) {
            Some(instance) => Ok(Arc::clone(instance)),
            None => {
                let store = Store::new(&JIT::new(Singlepass::new()).engine());
                let module = Module::from_binary(&store, wasm)
                    .map_err(|e| WasmError::Compile(e.to_string()))?;
                let env = Env::default();
                let import_object: ImportObject = import_builder(&store, &env);
                self.0.insert(
                    key,
                    Arc::new(Mutex::new(
                        Instance::new(&module, &import_object)
                            .map_err(|e| WasmError::Compile(e.to_string()))?,
                    )),
                );
                self.get(key, wasm, import_builder)
            }
        }
    }
}
