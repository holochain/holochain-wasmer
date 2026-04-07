use crate::prelude::*;
use std::sync::Arc;
use std::sync::OnceLock;
use wasmer::Engine;
use wasmer::Module;

/// A process-wide shared `wasmi` engine.
///
/// `wasmi` 1.x keeps a per-engine function-type registry and refuses to use a
/// module from one engine inside an instance/store backed by a different
/// engine — it panics with `encountered foreign entity in func type registry`.
/// Returning the same `Engine` from every call here keeps every module, store
/// and instance the host produces on a single registry.
fn shared_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(Engine::default)
}

/// Returns the shared engine that drives the pure-Rust `wasmi` interpreter.
///
/// The interpreter does not support metering.
pub fn make_engine() -> Engine {
    shared_engine().clone()
}

/// Runtime engine factory for the wasmi backend.
///
/// wasmi has no separate "headless" engine — modules and instances both run
/// against the same shared engine — so this just returns a clone of the
/// process-wide engine, identical to [`make_engine`]. It exists as a separate
/// function so that callers can pass it as the `make_runtime_engine`
/// parameter to [`crate::module::ModuleBuilder::new`].
pub fn make_runtime_engine() -> Engine {
    shared_engine().clone()
}

/// Build an interpreter module from wasm bytes.
pub fn build_module(wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
    let compiler_engine = make_engine();
    let res = Module::from_binary(&compiler_engine, wasm);
    let module = res.map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?;
    Ok(Arc::new(module))
}

#[cfg(test)]
mod tests {
    use super::build_module;

    #[test]
    fn build_module_test() {
        // simple example wasm taken from wasmer docs
        // https://docs.rs/wasmer/latest/wasmer/struct.Module.html#example
        let wasm: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7f,
            0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x0b, 0x01, 0x07, 0x61, 0x64, 0x64, 0x5f,
            0x6f, 0x6e, 0x65, 0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x41, 0x01,
            0x6a, 0x0b, 0x00, 0x1a, 0x04, 0x6e, 0x61, 0x6d, 0x65, 0x01, 0x0a, 0x01, 0x00, 0x07,
            0x61, 0x64, 0x64, 0x5f, 0x6f, 0x6e, 0x65, 0x02, 0x07, 0x01, 0x00, 0x01, 0x00, 0x02,
            0x70, 0x30,
        ];

        let res = build_module(wasm.as_slice());
        assert!(res.is_ok())
    }
}
