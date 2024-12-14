use crate::import::imports;
#[cfg(feature = "wasmer_wamr")]
use holochain_wasmer_host::module::build_module;
use holochain_wasmer_host::module::InstanceWithStore;
#[cfg(feature = "wasmer_sys")]
use holochain_wasmer_host::module::SerializedModuleCache;
use holochain_wasmer_host::prelude::*;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::sync::Arc;
#[cfg(feature = "wasmer_sys")]
use wasmer::wasmparser::Operator;
use wasmer::AsStoreMut;
#[cfg(feature = "wasmer_sys")]
use wasmer::CompilerConfig;
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
    Test,
    Memory,
}

#[cfg(feature = "wasmer_sys")]
pub static SERIALIZED_MODULE_CACHE: OnceCell<RwLock<SerializedModuleCache>> = OnceCell::new();
#[cfg(feature = "wasmer_sys")]
pub static SERIALIZED_MODULE_CACHE_UNMETERED: OnceCell<RwLock<SerializedModuleCache>> =
    OnceCell::new();

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
            TestWasm::Test => include_bytes!(concat!(
                env!("OUT_DIR"),
                "/wasm32-unknown-unknown/release/test_wasm.wasm"
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
            TestWasm::Test => "test",
            TestWasm::Memory => "memory",
        }
    }

    pub fn key(&self, metered: bool) -> [u8; 32] {
        match (self, metered) {
            (TestWasm::Empty, false) => [0; 32],
            (TestWasm::Empty, true) => [1; 32],
            (TestWasm::Io, false) => [2; 32],
            (TestWasm::Io, true) => [3; 32],
            (TestWasm::Test, false) => [4; 32],
            (TestWasm::Test, true) => [5; 32],
            (TestWasm::Memory, false) => [6; 32],
            (TestWasm::Memory, true) => [7; 32],
        }
    }

    #[cfg(feature = "wasmer_sys")]
    pub fn module_cache(&self, metered: bool) -> &OnceCell<RwLock<SerializedModuleCache>> {
        if metered {
            &SERIALIZED_MODULE_CACHE
        } else {
            &SERIALIZED_MODULE_CACHE_UNMETERED
        }
    }

    #[cfg(feature = "wasmer_sys")]
    pub fn module(&self, metered: bool) -> Arc<Module> {
        match self.module_cache(metered).get() {
            Some(cache) => cache.write().get(self.key(metered), self.bytes()).unwrap(),
            None => {
                let cranelift_fn = || {
                    let cost_function = |_operator: &Operator| -> u64 { 1 };
                    let metering = Arc::new(Metering::new(10_000_000_000, cost_function));
                    #[cfg(feature = "wasmer_sys_dev")]
                    let mut compiler = wasmer::Cranelift::default();
                    #[cfg(feature = "wasmer_sys_prod")]
                    let mut compiler = wasmer::LLVM::default();

                    compiler.canonicalize_nans(true);
                    compiler.push_middleware(metering);
                    Engine::from(compiler)
                };

                let compiler_fn_unmetered = || {
                    #[cfg(feature = "wasmer_sys_dev")]
                    let mut compiler = wasmer::Cranelift::default();
                    #[cfg(feature = "wasmer_sys_prod")]
                    let mut compiler = wasmer::LLVM::default();

                    compiler.canonicalize_nans(true);
                    Engine::from(compiler)
                };

                // This will error if the cache is already initialized
                // which could happen if two tests are running in parallel.
                // It doesn't matter which one wins, so we just ignore the error.
                let _did_init_ok = self.module_cache(metered).set(parking_lot::RwLock::new(
                    SerializedModuleCache::default_with_engine(
                        if metered {
                            cranelift_fn
                        } else {
                            compiler_fn_unmetered
                        },
                        None,
                    ),
                ));

                // Just recurse now that the cache is initialized.
                self.module(metered)
            }
        }
    }

    #[cfg(feature = "wasmer_wamr")]
    pub fn module(&self, _metered: bool) -> Arc<Module> {
        build_module(self.bytes()).unwrap()
    }

    pub fn _instance(&self, metered: bool) -> InstanceWithStore {
        let module = self.module(metered);
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
            if metered {
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
        }

        InstanceWithStore {
            store: Arc::new(Mutex::new(store)),
            instance: Arc::new(instance),
        }
    }

    #[cfg(feature = "wasmer_sys")]
    pub fn instance(&self) -> InstanceWithStore {
        self._instance(true)
    }

    #[cfg(feature = "wasmer_wamr")]
    pub fn instance(&self) -> InstanceWithStore {
        self.unmetered_instance()
    }

    pub fn unmetered_instance(&self) -> InstanceWithStore {
        self._instance(false)
    }
}
