use holochain_serialized_bytes::prelude::*;
use thiserror::Error;

/// Enum of all possible ERROR codes that a Zome API Function could return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[rustfmt::skip]
pub enum WasmErrorType {
    /// while converting pointers and lengths between u64 and i64 across the host/guest
    /// we hit either a negative number (cannot fit in u64) or very large number (cannot fit in i64)
    /// negative pointers and lengths are almost certainly indicative of a critical bug somewhere
    /// max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    /// if commercial hardware ever threatens to overstep this limit
    PointerMap,
    /// These bytes failed to deserialize.
    /// The host should provide nice debug info and context that the wasm guest won't have.
    Deserialize(SerializedBytesError),
    /// Something failed to serialize.
    /// This should be rare or impossible for basically everything that implements Serialize.
    Serialize(SerializedBytesError),
    /// Somehow we errored while erroring.
    /// For example, maybe we failed to serialize an error while attempting to serialize an error.
    ErrorWhileError,
    /// something went wrong while writing or reading bytes to/from wasm memory
    /// this means something like "reading 16 bytes did not produce 2x WasmSize ints"
    /// or maybe even "failed to write a byte to some pre-allocated wasm memory"
    /// whatever this is it is very bad and probably not recoverable
    Memory,
    /// failed to take bytes out of the guest and do something with it
    /// the string is whatever error message comes back from the interal process
    GuestResultHandling,
    /// something to do with zome logic that we don't know about
    Zome,
    /// somehow wasmer failed to compile machine code from wasm byte code
    Compile,

    CallError,
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
