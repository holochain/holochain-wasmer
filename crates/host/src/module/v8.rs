use crate::prelude::*;
use std::sync::Arc;
use std::sync::OnceLock;
use wasmer::Engine;
use wasmer::Module;
use wasmer::v8::V8;

/// A process-wide shared V8 engine.
///
/// V8 initializes a single runtime the first time `wasm_engine_new` is
/// called inside the wee8 C API. After that point most V8 flags are
/// frozen, and wasmer's V8 backend itself serializes access through a
/// global `OnceLock<Mutex<EngineCapsule>>`. Returning the same
/// [`Engine`] from every call here keeps every module, store and
/// instance the host produces aligned with that single runtime.
fn shared_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| V8::new().into())
}

/// Request a V8 flag string before the engine is constructed.
///
/// V8 takes its command-line flags via `v8::V8::SetFlagsFromString`,
/// which must be invoked before the runtime initializes. In practice
/// that means: call this before [`make_engine`] (or anything else
/// that builds an [`Engine`]) runs for the first time, otherwise the
/// flag will be silently ignored for values V8 locks at init.
///
/// The canonical caller is an iOS embedder passing `"--jitless"` —
/// Apple does not grant the JIT entitlement to non-browser apps, so
/// V8 must never request RWX pages. The shim accepts any flag string
/// V8 itself accepts on its command line.
pub fn set_flags_from_string(flags: &str) {
    V8::set_flags_from_string(flags);
}

/// Returns the shared engine that drives the V8 backend.
///
/// V8 supports gas-injection-style metering via a transform pass, but
/// the wasmer v8 backend doesn't expose the same `wasmer_middlewares`
/// metering API the `sys` backends use — so from the host's point of
/// view this engine runs unmetered today.
pub fn make_engine() -> Engine {
    shared_engine().clone()
}

/// Runtime engine factory for the V8 backend.
///
/// Like `wasmi`, V8 has no separate "headless" engine — modules and
/// instances both run against the same process-wide engine — so this
/// just returns a clone of the shared engine, identical to
/// [`make_engine`]. It exists as a separate function so that callers
/// can pass it as the `make_runtime_engine` parameter to
/// [`crate::module::ModuleBuilder::new`].
pub fn make_runtime_engine() -> Engine {
    shared_engine().clone()
}

/// Build a V8-backed module from wasm bytes.
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
