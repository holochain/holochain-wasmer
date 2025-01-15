use crate::import::imports;
#[cfg(feature = "wasmer_wamr")]
use holochain_wasmer_host::module::build_module;
use holochain_wasmer_host::module::InstanceWithStore;
use holochain_wasmer_host::module::ModuleBuilder;
use holochain_wasmer_host::module::ModuleCache;
use holochain_wasmer_host::prelude::*;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::sync::Arc;
#[cfg(feature = "wasmer_sys")]
use wasmer::wasmparser::Operator;
use wasmer::AsStoreMut;
#[cfg(feature = "wasmer_sys")]
use wasmer::CompilerConfig;
#[cfg(feature = "wasmer_sys")]
use wasmer::Engine;
use wasmer::FunctionEnv;
use wasmer::Imports;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
#[cfg(feature = "wasmer_sys")]
use wasmer_middlewares::Metering;

pub enum TestWasm {
    Empty,
    Io,
    Core,
    Memory,
}

pub static MODULE_CACHE: OnceCell<RwLock<ModuleCache>> = OnceCell::new();

impl TestWasm {
    pub fn bytes(&self) -> &[u8] {
        match self {
            TestWasm::Empty => include_bytes!(concat!(
                env!("OUT_DIR"),
                "/wasm32-unknown-unknown/release/test_wasm_empty.wasm"
            )),
            TestWasm::Io => include_bytes!(concat!(
                env!("OUT_DIR"),
                "/wasm32-unknown-unknown/release/test_wasm_io.wasm"
            )),
            TestWasm::Core => include_bytes!(concat!(
                env!("OUT_DIR"),
                "/wasm32-unknown-unknown/release/test_wasm_core.wasm"
            )),
            TestWasm::Memory => include_bytes!(concat!(
                env!("OUT_DIR"),
                "/wasm32-unknown-unknown/release/test_wasm_memory.wasm"
            )),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            TestWasm::Empty => "empty",
            TestWasm::Io => "io",
            TestWasm::Core => "core",
            TestWasm::Memory => "memory",
        }
    }

    pub fn key(&self) -> [u8; 32] {
        match self {
            TestWasm::Empty => [1; 32],
            TestWasm::Io => [2; 32],
            TestWasm::Core => [3; 32],
            TestWasm::Memory => [4; 32],
        }
    }

    #[cfg(feature = "wasmer_sys")]
    pub fn module(&self) -> Arc<Module> {
        match &MODULE_CACHE.get() {
            Some(cache) => cache.write().get(self.key(), self.bytes()).unwrap(),
            None => {
                // This will error if the cache is already initialized
                // which could happen if two tests are running in parallel.
                // It doesn't matter which one wins, so we just ignore the error.
                let _did_init_ok =
                    self.module_cache(metered)
                        .set(parking_lot::RwLock::new(ModuleCache::new(None)));

                // Just recurse now that the cache is initialized.
                self.module()
            }
        }
    }

    #[cfg(feature = "wasmer_wamr")]
    pub fn module(&self) -> Arc<Module> {
        build_module(self.bytes()).unwrap()
    }

    pub fn instance(&self) -> InstanceWithStore {
        let module = self.module();
        let mut store = Store::default();
        let function_env;
        let instance;
        {
            let mut store_mut = store.as_store_mut();
            function_env = FunctionEnv::new(&mut store_mut, Env::default());
            let built_imports: Imports = imports(&mut store_mut, &function_env);
            instance = Instance::new(&mut store_mut, &module, &built_imports).unwrap();
        }

        {
            let mut function_env_mut = function_env.into_mut(&mut store);
            let (data_mut, store_mut) = function_env_mut.data_and_store_mut();
            data_mut.memory = Some(instance.exports.get_memory("memory").unwrap().clone());
            data_mut.deallocate = Some(
                instance
                    .exports
                    .get_typed_function(&store_mut, "__hc__deallocate_1")
                    .unwrap(),
            );
            data_mut.allocate = Some(
                instance
                    .exports
                    .get_typed_function(&store_mut, "__hc__allocate_1")
                    .unwrap(),
            );

            #[cfg(feature = "wasmer_sys")]
            data_mut.wasmer_metering_points_exhausted = Some(
                instance
                    .exports
                    .get_global("wasmer_metering_points_exhausted")
                    .unwrap()
                    .clone(),
            );
            data_mut.wasmer_metering_remaining_points = Some(
                instance
                    .exports
                    .get_global("wasmer_metering_remaining_points")
                    .unwrap()
                    .clone(),
            );
        }

        InstanceWithStore {
            store: Arc::new(Mutex::new(store)),
            instance: Arc::new(instance),
        }
    }
}
