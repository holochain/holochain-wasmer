use crate::import::imports;
use holochain_wasmer_host::module::SerializedModuleCache;
use holochain_wasmer_host::module::SERIALIZED_MODULE_CACHE;
use holochain_wasmer_host::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use wasmer::wasmparser::Operator;
use wasmer::AsStoreMut;
use wasmer::CompilerConfig;
use wasmer::Cranelift;
use wasmer::Imports;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;
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

    pub fn module(&self, metered: bool) -> Arc<(Mutex<Store>, Module)> {
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
                    let (store, module) = SERIALIZED_MODULE_CACHE
                        .get()
                        .unwrap()
                        .write()
                        .get(self.key(metered), self.bytes())
                        .unwrap();
                    Arc::new((Mutex::new(store), module))
                }

                _ => unreachable!(),
            },
        }
    }

    pub fn _instance(&self, metered: bool) -> Arc<Mutex<Instance>> {
        let (store, module) = *self.module(metered);
        let mut store = store.get_mut();
        let env = Env::default();
        let imports: Imports = imports(&mut store.as_store_mut(), env);
        Arc::new(Mutex::new(
            Instance::new(&mut store.as_store_mut(), &module, &imports).unwrap(),
        ))
    }

    pub fn instance(&self) -> Arc<Mutex<Instance>> {
        self._instance(true)
    }

    pub fn unmetered_instance(&self) -> Arc<Mutex<Instance>> {
        self._instance(false)
    }
}
