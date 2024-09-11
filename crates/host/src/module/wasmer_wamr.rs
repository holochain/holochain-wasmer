use wasmer::Engine;

/// Generate an engine with a wasm interpreter
/// The interpreter used (wasm micro runtime) does not support metering
/// See tracking issue: https://github.com/bytecodealliance/wasm-micro-runtime/issues/2163
pub fn make_engine() -> Engine {
    Engine::default()
}

pub fn make_runtime_engine() -> Engine {
    Engine::default()
}
