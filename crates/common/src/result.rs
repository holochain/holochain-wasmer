use holochain_serialized_bytes::prelude::*;
use thiserror::Error;

/// Enum of all possible ERROR states that wasm can can encounter.
///
/// Used in [`wasm_error!`] for specifying the error type and message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WasmErrorInner {
    /// While moving pointers and lengths across the host/guest we hit an unsafe
    /// conversion such as a negative pointer or out of bounds value.
    PointerMap,
    /// These bytes failed to deserialize.
    /// The host should provide nice debug info and context that the wasm guest won't have.
    #[serde(with = "serde_bytes")]
    Deserialize(Box<[u8]>),
    /// Something failed to serialize.
    /// This should be rare or impossible for almost everything that implements `Serialize`.
    Serialize(SerializedBytesError),
    /// Somehow we errored while erroring.
    /// For example, maybe we failed to serialize an error while attempting to serialize an error.
    ErrorWhileError,
    /// Something went wrong while writing or reading bytes to/from wasm memory.
    /// Whatever this is it is very bad and probably not recoverable.
    Memory,
    /// Host failed to take bytes out of the guest and do something with it.
    /// The string is whatever error message comes back from the internal process.
    GuestResultHandling(String),
    /// Error with guest logic that the host doesn't know about.
    Guest(String),
    /// Error with host logic that the guest doesn't know about.
    Host(String),
    /// Something to do with host logic that the guest doesn't know about
    /// AND wasm execution MUST immediately halt.
    /// The `Vec<u8>` holds the encoded data as though the guest had returned.
    HostShortCircuit(Vec<u8>),
    /// Wasmer failed to compile machine code from wasm byte code.
    Compile(String),
    /// The host failed to call a function in the guest.
    CallError(String),
    /// Host attempted to interact with the module cache before it was initialized.
    UninitializedSerializedModuleCache,
}

impl WasmErrorInner {
    /// Some errors indicate the wasm guest is potentially corrupt and so the
    /// host MUST NOT reuse it (e.g. in a cache of wasm instances). Other errors
    /// MAY NOT invalidate an instance cache on the host.
    pub fn maybe_corrupt(&self) -> bool {
        match self {
            Self::PointerMap
            | Self::ErrorWhileError
            | Self::Memory
            | Self::GuestResultHandling(_)
            | Self::Compile(_)
            | Self::CallError(_)
            | Self::UninitializedSerializedModuleCache => true,
            Self::Deserialize(_)
            | Self::Serialize(_)
            | Self::Guest(_)
            | Self::Host(_)
            | Self::HostShortCircuit(_) => false,
        }
    }
}

impl From<std::num::TryFromIntError> for WasmErrorInner {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::PointerMap
    }
}

impl From<std::array::TryFromSliceError> for WasmErrorInner {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::Memory
    }
}

impl From<SerializedBytesError> for WasmErrorInner {
    fn from(error: SerializedBytesError) -> Self {
        Self::Serialize(error)
    }
}

/// Wraps a WasmErrorInner with a file and line number.
/// The easiest way to generate this is with the `wasm_error!` macro that will
/// insert the correct file/line and can create strings by forwarding args to
/// the `format!` macro.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Error)]
#[rustfmt::skip]
pub struct WasmError {
    pub file: String,
    pub line: u32,
    pub error: WasmErrorInner,
}

/// Helper macro for returning an error from a WASM.
///
/// Automatically included in the error are the file and the line number where the
/// error occurred. The error type is one of the [`WasmErrorInner`] variants.
///
/// This macro is the recommended way of returning an error from a Zome function.
///
/// If a single expression is passed to `wasm_error!` the result will be converted
/// to a `WasmErrorInner` via. `into()` so string and `WasmErrorInner` values are
/// both supported directly.
///
/// If a list of arguments is passed to `wasm_error!` it will first be forwarded
/// to `format!` and then the resultant string converted to `WasmErrorInner`.
///
/// As the string->WasmErrorInner conversion is handled by a call to into, the
/// feature `error_as_host` can be used so that `WasmErrorInner::Host` is produced
/// by the macro from any passed/generated string.
///
/// # Examples
///
/// ```ignore
/// Err(wasm_error!(WasmErrorInner::Guest("entry not found".to_string())));
/// Err(wasm_error!("entry not found"));
/// Err(wasm_error!("{} {}", "entry", "not found"));
/// Err(wasm_error!(WasmErrorInner::Host("some host error".into())));
/// ```
#[macro_export]
macro_rules! wasm_error {
    ($e:expr) => {
        WasmError {
            file: file!().to_string(),
            line: line!(),
            error: $e.into(),
        }
    };
    ($($arg:tt)*) => {{
        $crate::wasm_error!(std::format!($($arg)*))
    }};
}

impl From<WasmError> for String {
    fn from(e: WasmError) -> Self {
        format!("{}", e)
    }
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Allows ? in a TryFrom context downstream.
impl From<core::convert::Infallible> for WasmError {
    fn from(_: core::convert::Infallible) -> WasmError {
        unreachable!()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<WasmError> for wasmer_engine::RuntimeError {
    fn from(wasm_error: WasmError) -> wasmer_engine::RuntimeError {
        wasmer_engine::RuntimeError::user(Box::new(wasm_error))
    }
}

#[cfg(not(feature = "error_as_host"))]
impl From<String> for WasmErrorInner {
    fn from(s: String) -> Self {
        Self::Guest(s)
    }
}

#[cfg(feature = "error_as_host")]
impl From<String> for WasmErrorInner {
    fn from(s: String) -> Self {
        Self::Host(s)
    }
}

impl From<&str> for WasmErrorInner {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}
