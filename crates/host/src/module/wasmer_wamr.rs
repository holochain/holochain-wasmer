use crate::prelude::*;
use std::path::Path;
use std::sync::Arc;
use wasmer::CompileError;
use wasmer::DeserializeError;
use wasmer::Engine;
use wasmer::Module;

/// Take WASM binary and prepare a wasmer Module suitable for iOS
pub fn build_ios_module(_wasm: &[u8]) -> Result<Module, CompileError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Deserialize a previously compiled module for iOS from a file.
pub fn get_ios_module_from_file(_path: &Path) -> Result<Module, DeserializeError> {
    unimplemented!("The feature flag 'wasmer_sys' must be enabled to support compiling wasm");
}

/// Build an interpreter module from wasm bytes.
/// The interpreter used (wasm micro runtime) does not support metering
/// See tracking issue: https://github.com/bytecodealliance/wasm-micro-runtime/issues/2163
pub fn build_module(wasm: &[u8]) -> Result<Arc<Module>, wasmer::RuntimeError> {
    let compiler_engine = Engine::default();
    let res = Module::from_binary(&compiler_engine, wasm);
    let module = res.map_err(|e| wasm_error!(WasmErrorInner::ModuleBuild(e.to_string())))?;
    Ok(Arc::new(module))
}

#[cfg(test)]
pub mod tests {
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

