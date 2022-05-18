use crate::import::import_object;
use crate::wasmparser::Operator;
use holochain_wasmer_host::module::SerializedModuleCache;
use holochain_wasmer_host::module::SERIALIZED_MODULE_CACHE;
use holochain_wasmer_host::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
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

    pub fn key(&self) -> [u8; 32] {
        match self {
            TestWasm::Empty => [0; 32],
            TestWasm::Io => [1; 32],
            TestWasm::Test => [2; 32],
            TestWasm::Memory => [3; 32],
        }
    }

    pub fn module(&self) -> Arc<Module> {
        match MODULE_CACHE.write().get(self.key(), self.bytes()) {
            Ok(v) => v,
            Err(runtime_error) => match runtime_error.downcast::<WasmError>() {
                Ok(WasmError { error, .. }) => match error {
                    WasmErrorInner::UninitializedSerializedModuleCache => {
                        {
                            let cranelift_fn = || {
                                let cost_function = |_operator: &Operator| -> u64 { 1 };
                                let metering = Arc::new(Metering::new(10000000000, cost_function));
                                let mut cranelift = Cranelift::default();
                                cranelift.canonicalize_nans(true).push_middleware(metering);
                                cranelift
                            };

                            assert!(SERIALIZED_MODULE_CACHE
                                .set(parking_lot::RwLock::new(
                                    SerializedModuleCache::default_with_cranelift(cranelift_fn)
                                ))
                                .is_ok());
                        }
                        Arc::new(
                            SERIALIZED_MODULE_CACHE
                                .get()
                                .unwrap()
                                .write()
                                .get(self.key(), self.bytes())
                                .unwrap(),
                        )
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
        }
    }

    pub fn instance(&self) -> Arc<Mutex<Instance>> {
        let module = self.module();
        let env = Env::default();
        let import_object: ImportObject = import_object(&module.store(), &env);
        Arc::new(Mutex::new(Instance::new(&module, &import_object).unwrap()))
    }
}
