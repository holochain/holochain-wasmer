use holochain_wasmer_common::WasmError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Wraps a WasmErrorInner with a file and line number.
/// The easiest way to generate this is with the `wasm_error!` macro that will
/// insert the correct file/line and can create strings by forwarding args to
/// the `format!` macro.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Error)]
#[rustfmt::skip]
pub struct WasmHostError(pub WasmError);

impl std::fmt::Display for WasmHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<WasmError> for WasmHostError {
    fn from(wasm_error: WasmError) -> WasmHostError {
        WasmHostError(wasm_error)
    }
}

impl From<WasmHostError> for wasmer::RuntimeError {
    fn from(wasm_error: WasmHostError) -> wasmer::RuntimeError {
        wasmer::RuntimeError::user(Box::new(wasm_error.0))
    }
}

#[macro_export]
macro_rules! wasm_host_error {
    ($e:expr) => {
      WasmHostError(WasmError {
        // On Windows the `file!()` macro returns a path with inconsistent formatting:
        // from the workspace to the package root it uses backwards-slashes,
        // then within the package it uses forwards-slashes.
        // i.e. "test-crates\\wasm_core\\src/wasm.rs"
        //
        // To remedy this we normalize the formatting here.
          file: file!().replace('\\', "/").to_string(),
          line: line!(),
          error: $e.into(),
      })
    };
    ($($arg:tt)*) => {{
        $crate::wasm_host_error!(std::format!($($arg)*))
    }};
}
