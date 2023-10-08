use crate::import::imports;
use holochain_wasmer_host::module::InstanceWithStore;
use holochain_wasmer_host::module::ModuleWithStore;
use holochain_wasmer_host::module::SerializedModuleCache;
use holochain_wasmer_host::module::SERIALIZED_MODULE_CACHE;
use holochain_wasmer_host::prelude::*;
use std::sync::Arc;
use wasmer::wasmparser::Operator;
use wasmer::AsStoreMut;
use wasmer::CompilerConfig;
use wasmer::Cranelift;
use wasmer::FunctionEnv;
use wasmer::Imports;
use wasmer::Instance;
use wasmer_middlewares::Metering;

pub enum TestWasm {
    Empty,
    Io,
    Test,
    Memory,
}

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

    pub fn module(&self, metered: bool) -> Arc<ModuleWithStore> {
        match MODULE_CACHE.write().get(self.key(metered), self.bytes()) {
            Ok(v) => v,
            Err(runtime_error) => match runtime_error.downcast::<WasmError>() {
                Ok(WasmError {
                    error: WasmErrorInner::UninitializedSerializedModuleCache,
                    ..
                }) => {
                    {
                        let cranelift_fn = || {
                            let cost_function = |_operator: &Operator| -> u64 { 1 };
                            let metering = Arc::new(Metering::new(10000000000, cost_function));
                            let mut cranelift = Cranelift::default();
                            cranelift.canonicalize_nans(true).push_middleware(metering);
                            cranelift
                        };

                        let cranelift_fn_unmetered = || {
                            let mut cranelift = Cranelift::default();
                            cranelift.canonicalize_nans(true);
                            cranelift
                        };

                        assert!(SERIALIZED_MODULE_CACHE
                            .set(parking_lot::RwLock::new(
                                SerializedModuleCache::default_with_cranelift(if metered {
                                    cranelift_fn
                                } else {
                                    cranelift_fn_unmetered
                                })
                            ))
                            .is_ok());
                    }
                    SERIALIZED_MODULE_CACHE
                        .get()
                        .unwrap()
                        .write()
                        .get(self.key(metered), self.bytes())
                        .unwrap()
                }

                _ => unreachable!(),
            },
        }
    }

    pub fn _instance(&self, metered: bool) -> InstanceWithStore {
        let module_with_store = self.module(metered);
        let function_env;
        let instance;
        {
            let mut store_lock = module_with_store.store.lock();
            let mut store_mut = store_lock.as_store_mut();
            function_env = FunctionEnv::new(&mut store_mut, Env::default());
            let built_imports: Imports = imports(&mut store_mut, &function_env);
            instance =
                Instance::new(&mut store_mut, &module_with_store.module, &built_imports).unwrap();
        }

        {
            let mut store_lock = module_with_store.store.lock();
            let mut function_env_mut = function_env.into_mut(&mut store_lock);
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
        }

        InstanceWithStore {
            store: module_with_store.store.clone(),
            instance: Arc::new(instance),
        }
    }

    pub fn instance(&self) -> InstanceWithStore {
        self._instance(true)
    }

    pub fn unmetered_instance(&self) -> InstanceWithStore {
        self._instance(false)
    }
}
