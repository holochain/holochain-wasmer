use holochain_serialized_bytes::prelude::*;
use thiserror::Error;

/// Enum of all possible ERROR codes that a Zome API Function could return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[rustfmt::skip]
pub enum WasmErrorType {
    /// While converting pointers and lengths between u64 and i64 across the host/guest.
    /// Either a negative number (cannot fit in u64) or very large number (cannot fit in i64).
    /// Negative pointers and lengths are almost certainly indicative of a critical bug somewhere.
    /// Max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    /// if commercial hardware ever threatens to overstep this limit.
    PointerMap,
    /// These bytes failed to deserialize.
    /// SerializedBytesError provides nice debug info about the deserialization itself.
    /// Please provide additional context about what was happening when deserialization failed.
    Deserialize(SerializedBytesError),
    /// Something failed to serialize.
    /// This should be rare or impossible for basically everything that implements Serialize.
    /// SerializedBytesError provides nice debug info about the serialization itself.
    /// Please provide additional context about what was happening when serializaiton failed.
    Serialize(SerializedBytesError),
    /// Somehow we errored while erroring.
    /// For example, maybe we failed to serialize an error while attempting to serialize an error.
    ErrorWhileError,
    /// Something went wrong while writing or reading bytes to/from wasm memory.
    /// This means something like "reading 16 bytes did not produce 2x WasmSize ints"
    /// or maybe even "failed to write a byte to some pre-allocated wasm memory".
    /// Whatever this is it is very bad and probably not recoverable.
    Memory,
    /// Somehow wasmer failed to compile machine code from wasm byte code.
    Compile,
    /// Somehow wasmer failed to call a function on the wasm instance.
    Call,

    /// Generic error from the guest.
    Guest,
    /// Generic error from the host.
    Host,
}

impl std::fmt::Display for WasmErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Representation of message to be logged via the `debug` host function
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Error)]
pub struct WasmError {
    pub error_type: WasmErrorType,
    pub module_path: String,
    pub file: String,
    pub line: u32,
    pub msg: String,
}

impl WasmError {
    /// Constructor
    pub fn new<S: ToString>(error_type: WasmErrorType, msg: S) -> Self {
        Self {
            module_path: module_path!().to_string(),
            file: file!().to_string(),
            line: line!(),
            error_type,
            msg: msg.to_string(),
        }
    }

    pub fn from_guest<S: ToString>(msg: S) -> Self {
        Self::new(WasmErrorType::Guest, msg)
    }

    pub fn from_host<S: ToString>(msg: S) -> Self {
        Self::new(WasmErrorType::Host, msg)
    }

    pub fn as_error_type(&self) -> &WasmErrorType {
        &self.error_type
    }

    /// Access the msg part
    pub fn as_msg(&self) -> &str {
        &self.msg
    }

    /// Access the module_path part
    pub fn as_module_path(&self) -> &str {
        &self.module_path
    }

    /// Access the file part
    pub fn as_file(&self) -> &str {
        &self.file
    }

    /// Access the line part
    pub fn as_line(&self) -> u32 {
        self.line
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

impl From<WasmError> for String {
    fn from(e: WasmError) -> Self {
        format!("{}", e)
    }
}

impl From<std::array::TryFromSliceError> for WasmError {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::new(
            WasmErrorType::Memory,
            String::from("Failed to convert a byte slice during memory handling"),
        )
    }
}

impl From<std::num::TryFromIntError> for WasmError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::new(
            WasmErrorType::PointerMap,
            String::from("Failed to convert between integer types"),
        )
    }
}

/// A generic conversion of serialization errors to WasmError.
/// This is a double edged sword.
/// On one hand it allows us to use `?` which is great for encode/decode in app code.
/// On the other hand it is a missed opportunity to provide additional debugging context in the
/// debug message, where serialization/deserialization is exactly where we most often need that
/// additonal context because often the compiler does not have it either.
/// If you find your way to this code path and wish you had additional context, please provide it!
impl From<SerializedBytesError> for WasmError {
    fn from(serialized_bytes_error: SerializedBytesError) -> Self {
        match serialized_bytes_error {
            SerializedBytesError::Serialize(_, _) => WasmError::new(
                WasmErrorType::Serialize(serialized_bytes_error),
                "Failed to serialize.",
            ),
            SerializedBytesError::Deserialize(_, _) => WasmError::new(
                WasmErrorType::Deserialize(serialized_bytes_error),
                "Failed to deserialize.",
            ),
        }
    }
}
