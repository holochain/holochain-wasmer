use crate::result;
use crate::GuestPtr;
use crate::Len;
use crate::WasmSize;
use byte_slice_cast::AsSliceOf;
use std::convert::TryInto;

pub const WASM_FAT_PTR_ITEMS: usize = 2;

/// WasmFatPtr is a 2 item WasmSize array of offset/length
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
/// @see https://iandouglasscott.com/2018/05/28/exploring-rust-fat-pointers/
///
/// @todo is this last point definitely true?
/// it seems that WasmPtr<u8, Array> could potentially be used here as the compiler/allocator risk
/// looks similar to what we're doing anyway
#[repr(transparent)]
pub struct WasmFatPtr([WasmSize; WASM_FAT_PTR_ITEMS]);

impl WasmFatPtr {
    pub fn ptr_offset(&self) -> GuestPtr {
        (self.0)[0]
    }

    pub fn ptr_len(&self) -> Len {
        (self.0)[1]
    }
}

impl From<[WasmSize; WASM_FAT_PTR_ITEMS]> for WasmFatPtr {
    fn from(array: [WasmSize; WASM_FAT_PTR_ITEMS]) -> Self {
        Self(array)
    }
}

impl std::convert::TryFrom<Vec<u8>> for WasmFatPtr {
    type Error = result::WasmError;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(v.as_slice_of::<GuestPtr>()?.try_into()?))
    }
}

pub const WASM_FAT_PTR_BYTES_LEN: usize = std::mem::size_of::<WasmSize>() * 2;
/// Need WasmFatPtr to be a u8 array to copy as bytes across host/guest
/// the WasmSize integers of a WasmPtr are broken down into u8 bytes to copy into wasm memory
pub type WasmFatPtrBytes = [u8; WASM_FAT_PTR_BYTES_LEN];
