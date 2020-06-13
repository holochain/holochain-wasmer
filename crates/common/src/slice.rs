use crate::result;
use crate::GuestPtr;
use crate::Len;
use crate::WasmSize;
use byte_slice_cast::AsSliceOf;
use std::convert::TryInto;

pub const WASM_SLICE_ITEMS: usize = 2;
pub const WASM_SLICE_BYTES: usize = std::mem::size_of::<WasmSize>() * WASM_SLICE_ITEMS;

/// WasmSlice is a 2 item WasmSize array of offset/length
/// exists so that the host can co-ordinate linear memory with the guest without over reliance on
/// compiler/allocation specific implementation details that could change over time
///
/// the offset always represents a position in wasm linear memory _never_ on the host
/// the length always represents u8 bytes _not_ items
///
/// we do this instead of sharing Box or slices or WasmPtr or whatever across the host/guest
/// boundary because those abstractions all rely on internal details of the rust compiler that
/// potentially can change across compiler versions and between the host/guest
///
/// - WasmPtr: only stores the offset, relies on length to be provided or calculated from compiler
/// - Box: similar to WasmPtr in that it calculates lengths from compiler-specific information
/// - slice: works with usize instead of u32 so won't work on the host and relies on internal
///          representation of offset/length not changing
/// - Vec<u8>: rust docs explicitly state to not rely on the internal representation of vectors
/// - struct/enum: definitely subject to internal representation changing across compiler versions
/// - (offset, length) tuple: still just a form of struct, less reliable than an array
///
/// @see https://iandouglasscott.com/2018/05/28/exploring-rust-fat-pointers/
///
/// using a transparent newtype around an array of known length in bytes allows both the host and
/// guest to agree that there is a WasmSize offset/length pair located at a specific point in the
/// wasm guest's memory
#[repr(transparent)]
pub struct WasmSlice([WasmSize; WASM_SLICE_ITEMS]);

impl WasmSlice {
    pub fn ptr(&self) -> GuestPtr {
        (self.0)[0]
    }

    pub fn len(&self) -> Len {
        (self.0)[1]
    }
}

/// wraps a naked array in a WasmSlice newtype for type safety
impl From<[WasmSize; WASM_SLICE_ITEMS]> for WasmSlice {
    fn from(array: [WasmSize; WASM_SLICE_ITEMS]) -> Self {
        Self(array)
    }
}

/// attempts to take a vector of bytes of length WASM_SLICE_BYTES_LEN and convert it to a WasmSlice
/// this will fail if the vector is the wrong length to correctly build a WasmSlice array
impl std::convert::TryFrom<Vec<u8>> for WasmSlice {
    type Error = result::WasmError;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(v.as_slice_of::<GuestPtr>()?.try_into()?))
    }
}
