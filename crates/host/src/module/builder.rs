use crate::prelude::*;
use bytes::Bytes;
use std::sync::Arc;
use wasmer::{Engine, Module};

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
    /// Construct a `ModuleBuilder` for a particular wasmer backend.
    ///
    /// `make_engine` is invoked every time a new module is compiled
    /// (which on the sys backend means every time the cache misses).
    /// `make_runtime_engine` is invoked once at construction to produce
    /// the engine that deserialised modules are bound to.
    ///
    /// The two factories are passed in explicitly so that callers can
    /// pick a backend at the call site — for example
    /// `ModuleBuilder::new(sys::make_cranelift_engine, sys::make_runtime_engine)`
    /// for the cranelift-flavoured sys backend, or
    /// `ModuleBuilder::new(wasmi::make_engine, wasmi::make_runtime_engine)`
    /// for the wasmi interpreter. Both backend feature flags can be
    /// enabled simultaneously and the choice is made here at runtime.
    pub fn new(make_engine: fn() -> Engine, make_runtime_engine: fn() -> Engine) -> Self {
        Self {
            make_engine,
            runtime_engine: make_runtime_engine(),
        }
    }

    /// Build a Module from raw wasm bytes.
    ///
    /// `wasmer::Module::from_binary` performs full WebAssembly spec
    /// validation as part of module construction — the sys backend
    /// validates while compiling, and the wasmi backend validates inside
    /// `wasmi` itself. We therefore do **not** need (and should not add)
    /// a separate `Module::validate` call before this: it would reparse
    /// the module for nothing, and on wasmi `Module::validate` is
    /// `unimplemented!()` and would panic.
    ///
    /// Do not reach for `Module::from_binary_unchecked` — that is the
    /// explicit "skip validation" escape hatch and is only safe for wasm
    /// that has already been validated out-of-band.
    pub fn from_binary(&self, wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
        let compiler_engine = (self.make_engine)();
        let module = Arc::new(
            Module::from_binary(&compiler_engine, wasm)
                .map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?,
        );
        Ok(module)
    }

    /// Build a Module from a previously-serialized artifact.
    ///
    /// # Safety and trust model
    ///
    /// `wasmer::Module::deserialize` is documented as inherently
    /// **unsafe**: the bytes it loads contain pre-compiled machine code,
    /// and it is the caller's responsibility to guarantee they were
    /// produced by a matching `Module::serialize` call and have not been
    /// tampered with in between. Wasmer performs no spec-level
    /// revalidation here — the wasm was already validated when the
    /// artifact was first built.
    ///
    /// This function is only called from `ModuleCache::get` on the
    /// filesystem-cache hit branch. The caller is therefore trusting
    /// whatever lives at the cache path; the embedder is responsible for
    /// protecting that directory from other writers. Corrupt or
    /// version-mismatched files are handled by the cache: on deserialize
    /// failure the file is evicted and the module is rebuilt from the
    /// original wasm, which re-runs the validating path in
    /// [`Self::from_binary`]. Tampering that still happens to produce a
    /// deserializable artifact is *not* detected here.
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
