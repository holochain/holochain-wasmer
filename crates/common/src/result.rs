use holochain_json_api::{error::JsonError, json::JsonString};
use holochain_json_derive::DefaultJson;

/// Enum of all possible ERROR codes that a Zome API Function could return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultJson, PartialOrd, Ord, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum WasmError {
    Unspecified,
    ArgumentDeserializationFailed,
    OutOfMemory,
    ReceivedWrongActionResult,
    CallbackFailed,
    RecursiveCallForbidden,
    ResponseSerializationFailed,
    NotAnAllocation,
    ZeroSizedAllocation,
    UnknownEntryType,
    MismatchWasmCallDataType,
    EntryNotFound,
    WorkflowFailed,
    /// while converting pointers and lengths between u64 and i64 across the host/guest
    /// we hit either a negative number (cannot fit in u64) or very large number (cannot fit in i64)
    /// negative pointers and lengths are almost certainly indicative of a critical bug somewhere
    /// max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    /// if commercial hardware ever threatens to overstep this limit
    PointerMap,
    /// while shuffling raw bytes back and forward between Vec<u8> and utf-8 str values we have hit
    /// an invalid utf-8 string.
    /// in normal operations this is always a critical bug as the same rust internals are
    /// responsible for bytes and utf-8 in both directions.
    /// it is also possible that someone tries to throw invalid utf-8 at us to be evil.
    Utf8,
    /// similar to Utf8 we have somehow hit a struct that isn't round-tripping through JsonString
    /// correctly, which should be impossible for a well-behaved JsonStringAble thingie
    Json,
    /// something went wrong while writing or reading bytes to/from wasm memory
    /// this means something like "reading 16 bytes did not produce 2x u64 ints"
    /// or maybe even "failed to write a byte to some pre-allocated wasm memory"
    /// whatever this is it is very bad and probably not recoverable
    Memory,
    /// failed to take bytes out of the guest and do something with it
    /// the most common reason is a bad deserialization
    /// the string is whatever error message comes back from the interal process (e.g. a JsonError)
    GuestResultHandling(String),
    /// something to do with zome logic that we don't know about
    Zome(String),
}

impl From<std::num::TryFromIntError> for WasmError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::PointerMap
    }
}

impl From<std::str::Utf8Error> for WasmError {
    fn from(_: std::str::Utf8Error) -> Self {
        Self::Utf8
    }
}

impl From<byte_slice_cast::Error> for WasmError {
    fn from(_: byte_slice_cast::Error) -> Self {
        Self::Memory
    }
}

impl From<std::array::TryFromSliceError> for WasmError {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::Memory
    }
}

#[derive(Debug, Serialize, Deserialize, DefaultJson)]
pub enum WasmResult {
    Ok(JsonString),
    Err(WasmError),
}
