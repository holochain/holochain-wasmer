use std::sync::Arc;
// use std::sync::RwLock;
// use parking_lot::RwLock;
use wasmer::HostEnvInitError;
use wasmer::Instance;
use wasmer::LazyInit;
use wasmer::Memory;
use wasmer::WasmerEnv;

#[derive(Clone, Default, WasmerEnv)]
pub struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    // pub host_return_encoded: Arc<RwLock<Vec<u8>>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            memory: LazyInit::new(),
        }
    }
}

// impl WasmerEnv for Env {
//     fn init_with_instance(&mut self, instance: &Instance) -> Result<(), HostEnvInitError> {
//         dbg!(&self);
//         dbg!(&instance.exports);
//         if self.memory.get_ref().is_none() {
//             let memory = instance.exports.get_memory("memory").unwrap();
//             self.memory.initialize(memory.to_owned());
//             // dbg!("fooo", &memory);
//         }
//         dbg!(self.memory.get_ref());
//         // (*self.host_return_encoded.write()).push(0);
//         Ok(())
//     }
// }

// impl Env {
//     pub fn memory_ref(&self) -> Option<&Memory> {
//         let maybe_memory = self.memory.read();
//         dbg!(&maybe_memory);
//         maybe_memory.get_ref()
//     }
// }
