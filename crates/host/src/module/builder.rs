use crate::prelude::*;
use bytes::Bytes;
use std::sync::Arc;
use wasmer::{Engine, Module};

#[cfg(feature = "wasmer_sys")]
pub use super::wasmer_sys::*;

#[cfg(feature = "wasmer_wamr")]
pub use super::wasmer_wamr::*;

/// Responsible for storing the wasmer Engine used to build wasmer Modules.
#[derive(Debug)]
pub struct ModuleBuilder {
    // A function to create a new Engine for every module
    make_engine: fn() -> Engine,

    // The runtime engine is used only to execute function calls on instances,
    // so it does not require a compiler.
    runtime_engine: Engine,
}

impl ModuleBuilder {
    pub fn new() -> Self {
        Self {
            make_engine,
            runtime_engine: make_runtime_engine(),
        }
    }

    pub fn from_binary(&self, wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
        let compiler_engine = (self.make_engine)();
        let module = Arc::new(
            Module::from_binary(&compiler_engine, wasm)
                .map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?,
        );
        Ok(module)
    }

    pub fn from_serialized_module(
        &self,
        serialized_module: Bytes,
    ) -> Result<Arc<Module>, wasmer::RuntimeError> {
        let module = Arc::new(unsafe {
            Module::deserialize(&self.runtime_engine, serialized_module.clone())
                .map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?
        });
        Ok(module)
    }
}
