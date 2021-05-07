use crate::import::import_object;
use holochain_wasmer_host::prelude::*;
use wasmer::Module;

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

    pub fn module(&self) -> Module {
        MODULE_CACHE.lock().get(self.key(), self.bytes()).unwrap()
    }

    pub fn instance(&self) -> Instance {
        let module = self.module();
        let import_object: ImportObject = import_object(&module.store(), &Env::default());
        Instance::new(&module, &import_object).unwrap()
    }
}
