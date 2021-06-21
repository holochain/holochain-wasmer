pub mod result;
#[cfg(feature = "scopetracker_allocator")]
pub mod scopetracker;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;
pub use serde_bytes;

/// something like usize for wasm
/// wasm has a memory limit of 4GB so offsets and lengths fit in u32
///
/// when rust compiles to the wasm32-unknown-unknown target this means that usize will be u32 in
/// wasm but the host could interpret usize as either u32 or u64.
///
/// we need the host and the guest to have a shared agreement on the size of an offset/length or
/// the host will not be able to directly manipulate the host memory as it needs to
///
/// wasmer itself uses u32 in the WasmPtr abstraction etc.
/// @see https://docs.rs/wasmer-runtime/0.17.0/wasmer_runtime/struct.WasmPtr.html
pub type WasmSize = u32;

/// a WasmSize integer that represents the size of bytes to read/write to memory in direct
/// manipulations
pub type Len = WasmSize;

/// a WasmSize integer that points to a position in wasm linear memory that the host and guest are
/// sharing to communicate across function calls
pub type GuestPtr = WasmSize;

pub type GuestPtrLen = u64;

pub fn split_u64(u: GuestPtrLen) -> (GuestPtr, Len) {
    let bytes = u.to_le_bytes();
    (
        GuestPtr::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        Len::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
    )
}

pub fn merge_u64(guest_ptr: GuestPtr, len: Len) -> GuestPtrLen {
    let guest_ptr_bytes = guest_ptr.to_le_bytes();
    let len_bytes = len.to_le_bytes();
    GuestPtrLen::from_le_bytes([
        guest_ptr_bytes[0],
        guest_ptr_bytes[1],
        guest_ptr_bytes[2],
        guest_ptr_bytes[3],
        len_bytes[0],
        len_bytes[1],
        len_bytes[2],
        len_bytes[3],
    ])
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let guest_ptr = 9000000 as GuestPtr;
        let len = 1000 as GuestPtr;

        let (out_guest_ptr, out_len) = split_u64(merge_u64(guest_ptr, len));

        assert_eq!(guest_ptr, out_guest_ptr,);

        assert_eq!(len, out_len,);
    }
}
