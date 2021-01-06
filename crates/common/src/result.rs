use holochain_serialized_bytes::prelude::*;
use thiserror::Error;

/// Enum of all possible ERROR codes that a Zome API Function could return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, SerializedBytes, Error)]
#[rustfmt::skip]
pub enum WasmError {
    /// while converting pointers and lengths between u64 and i64 across the host/guest
    /// we hit either a negative number (cannot fit in u64) or very large number (cannot fit in i64)
    /// negative pointers and lengths are almost certainly indicative of a critical bug somewhere
    /// max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    /// if commercial hardware ever threatens to overstep this limit
    PointerMap,
    /// similar to Utf8 we have somehow hit a struct that isn't round-tripping through SerializedBytes
    /// correctly, which should be impossible for well behaved serialization
    SerializedBytes(SerializedBytesError),
    /// something went wrong while writing or reading bytes to/from wasm memory
    /// this means something like "reading 16 bytes did not produce 2x WasmSize ints"
    /// or maybe even "failed to write a byte to some pre-allocated wasm memory"
    /// whatever this is it is very bad and probably not recoverable
    Memory,
    /// failed to take bytes out of the guest and do something with it
    /// the string is whatever error message comes back from the interal process
    GuestResultHandling(String),
    /// something to do with zome logic that we don't know about
    Zome(String),
    /// somehow wasmer failed to compile machine code from wasm byte code
    Compile(String),

    CallError(String),
}

impl From<WasmError> for String {
    fn from(e: WasmError) -> Self {
        format!("{}", e)
    }
}

impl From<std::num::TryFromIntError> for WasmError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::PointerMap
    }
}

impl From<std::array::TryFromSliceError> for WasmError {
    fn from(_: std::array::TryFromSliceError) -> Self {
        Self::Memory
    }
}

impl From<SerializedBytesError> for WasmError {
    fn from(error: SerializedBytesError) -> Self {
        Self::SerializedBytes(error)
    }
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize, SerializedBytes)]
pub enum WasmResult {
    Ok(SerializedBytes),
    Err(WasmError),
}

impl From<core::convert::Infallible> for WasmError {
    fn from(_: core::convert::Infallible) -> WasmError {
        unreachable!()
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    fn wasm_result_serialized_bytes_round_trip() {
        #[derive(Clone, PartialEq, Debug, Serialize, Deserialize, SerializedBytes)]
        struct Foo(String);

        let foo = Foo(String::from("bar"));

        let wasm_result = WasmResult::Ok(foo.clone().try_into().unwrap());

        let wasm_result_sb = SerializedBytes::try_from(wasm_result).unwrap();

        let wasm_result_recover =
            WasmResult::try_from(wasm_result_sb).expect("could not restore wasm result");

        match wasm_result_recover {
            WasmResult::Ok(sb) => {
                let foo_recover = Foo::try_from(sb).expect("could not restore foo result");
                assert_eq!(foo, foo_recover);
            }
            _ => unreachable!(),
        };
    }
}
