use parking_lot::RwLock;
use std::sync::Arc;
use wasmer::LazyInit;
use wasmer::Memory;
use wasmer::WasmerEnv;

#[derive(Clone, Default, WasmerEnv)]
pub struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    pub(crate) host_return_encoded: Arc<RwLock<Vec<u8>>>,
}
