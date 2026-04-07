use holochain_wasmer_common::WasmError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Host-side wrapper around [`WasmError`].
///
/// This is the host-side counterpart to [`wasm_error!`](holochain_wasmer_common::wasm_error)
/// and exists so that the common crate doesn't need a direct `wasmer`
/// dependency just to convert errors into [`wasmer::RuntimeError`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Error)]
#[rustfmt::skip]
pub struct WasmHostError(pub WasmError);

impl std::fmt::Display for WasmHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
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
            module_path: ::core::module_path!().to_string(),
            line: ::core::line!(),
            error: $e.into(),
        })
    };
    ($($arg:tt)*) => {{
        $crate::wasm_host_error!(std::format!($($arg)*))
    }};
}
