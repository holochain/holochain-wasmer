//! Wasmer Host Module Manager
//!
//! This provides two ways to build & access wasm modules:
//!
//! 1. When using the feature flag `wasmer_sys`, modules can be accessed via the exported [`ModuleCache`].
//!    This ensures that wasm modules are compiled once, then cached and stored efficiently.
//!
//! 2. When using the feature flag `wasmer_wamr`, modules can be built via the exported [`build_module`] function.
//!    There is no need for caching in this case, as the wasm module is interpreted.

use parking_lot::Mutex;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Module;
use wasmer::Store;

#[cfg(feature = "wasmer_sys")]
mod wasmer_sys;
#[cfg(feature = "wasmer_sys")]
pub use wasmer_sys::*;

#[cfg(feature = "wasmer_wamr")]
mod wasmer_wamr;
#[cfg(feature = "wasmer_wamr")]
pub use wasmer_wamr::*;

#[derive(Clone, Debug)]
pub struct ModuleWithStore {
    pub store: Arc<Mutex<Store>>,
    pub module: Arc<Module>,
}

#[derive(Clone, Debug)]
pub struct InstanceWithStore {
    pub store: Arc<Mutex<Store>>,
    pub instance: Arc<Instance>,
}
