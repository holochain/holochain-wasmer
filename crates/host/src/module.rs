use crate::prelude::*;
use holochain_wasmer_common::WasmError;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
use wasmer::JIT;
use wasmer_compiler_singlepass::Singlepass;

pub struct ModuleCache(HashMap<[u8; 32], Arc<Module>>);
pub struct InstanceCache(HashMap<[u8; 32], Arc<Mutex<Instance>>>);

static COUNTER: Lazy<RwLock<u32>> = Lazy::new(|| RwLock::new(0));
// static STORE: Lazy<Store> = Lazy::new(|| Store::new(&JIT::new(Singlepass::new()).engine()));
pub static MODULE_CACHE: Lazy<RwLock<ModuleCache>> = Lazy::new(|| RwLock::new(ModuleCache::new()));
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

impl ModuleCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&mut self, key: [u8; 32], wasm: &[u8]) -> Result<Arc<Module>, WasmError> {
        dbg!(&key);
        match self.0.get(&key) {
            Some(module) => {
                *COUNTER.write() += 1;
                dbg!("hit");
                dbg!(&COUNTER);
                dbg!(self.0.keys().len());
                dbg!(Arc::strong_count(&module));
                dbg!(std::mem::size_of_val(&module.store()));
                Ok(Arc::clone(module))
            }
            None => {
                dbg!("miss");
                let module = Arc::new(
                    Module::from_binary(&Store::new(&JIT::new(Singlepass::new()).engine()), wasm)
                        .map_err(|e| WasmError::Compile(e.to_string()))?,
                );
                // dbg!(std::mem::size_of_val(wasm));
                dbg!(std::mem::size_of_val(&module));
                self.0.insert(key, Arc::clone(&module));
                // self.get(key, wasm)
                Ok(Arc::clone(&module))
            }
        }
    }
}
