use holochain_serialized_bytes::prelude::*;
use thiserror::Error;

/// Enum of all possible ERROR codes that a Zome API Function could return.
///
/// Used in [`wasm_error!`] for specifying the error type and message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WasmErrorInner {
    /// while converting pointers and lengths between u64 and i64 across the host/guest
    /// we hit either a negative number (cannot fit in u64) or very large number (cannot fit in i64)
    /// negative pointers and lengths are almost certainly indicative of a critical bug somewhere
    /// max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    /// if commercial hardware ever threatens to overstep this limit
    PointerMap,
    /// These bytes failed to deserialize.
    /// The host should provide nice debug info and context that the wasm guest won't have.
    #[serde(with = "serde_bytes")]
    Deserialize(Box<[u8]>),
    /// Something failed to serialize.
    /// This should be rare or impossible for basically everything that implements Serialize.
    Serialize(SerializedBytesError),
    /// Somehow we errored while erroring.
    /// For example, maybe we failed to serialize an error while attempting to serialize an error.
    ErrorWhileError,
    /// Something went wrong while writing or reading bytes to/from wasm memory.
    /// this means something like "reading 16 bytes did not produce 2x WasmSize ints"
    /// or maybe even "failed to write a byte to some pre-allocated wasm memory"
    /// whatever this is it is very bad and probably not recoverable
    Memory,
    /// Failed to take bytes out of the guest and do something with it.
    /// The string is whatever error message comes back from the interal process.
    GuestResultHandling(String),
    /// Something to do with guest logic that we don't know about
    Guest(String),
    /// Something to do with host logic that we don't know about
    Host(String),
    /// Something to do with host logic that we don't know about
    /// AND wasm execution MUST immediately halt.
    /// The Vec<u8> holds the encoded data as though the guest had returned.
    HostShortCircuit(Vec<u8>),
    /// Somehow wasmer failed to compile machine code from wasm byte code
    Compile(String),

    CallError(String),

    UninitializedSerializedModuleCache,
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
/// # Example
///
/// ```ignore
/// Err(wasm_error!(WasmErrorInner::Guest("entry not found".to_string())))
/// ```
#[macro_export]
macro_rules! wasm_error {
    ($e:expr) => {
        WasmError {
            file: file!().to_string(),
            line: line!(),
            error: $e,
        }
    };
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
