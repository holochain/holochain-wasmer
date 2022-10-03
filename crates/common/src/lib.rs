pub mod result;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;
pub use serde_bytes;

/// Something like `usize` for wasm.
/// Wasm has a memory limit of 4GB so offsets and lengths fit in `u32`.
///
/// When rust compiles to the `wasm32-unknown-unknown` target `usize` will be `u32` in
/// wasm but the host could interpret `usize` as either `u32` or `u64`. For that reason
/// we specify `u32` everywhere we need the host and the guest to have a shared agreement
/// on the size of an offset/length or the host will not be able to directly
/// manipulate the guest memory as it needs to.
///
/// Wasmer itself uses `u32` in the `WasmPtr` abstraction etc.
/// @see https://docs.rs/wasmer-runtime/0.17.0/wasmer_runtime/struct.WasmPtr.html
pub type WasmSize = u32;

/// A `WasmSize` that points to a position in wasm linear memory that the host
/// and guest are sharing to communicate across function calls.
pub type GuestPtr = WasmSize;

/// A `WasmSize` integer that represents the size of bytes to read/write to memory.
pub type Len = WasmSize;

/// Enough bits to fit a pointer and length into so we can return it. The externs
/// defined as "C" don't support multiple return values (unlike wasm). The native
/// Rust support for wasm externs is not stable at the time of writing.
pub type GuestPtrLen = u64;

/// Given a pointer and a length, return a `u64` merged `GuestPtrLen`.
/// Works via. a simple bitwise shift to move the pointer to high bits then OR
/// the length into the low bits.
pub fn merge_u64(guest_ptr: GuestPtr, len: Len) -> GuestPtrLen {
    // It should be impossible to hit these unwrap/panic conditions but it's more
    // conservative to define them than rely on an `as uX` cast.
    (u64::try_from(guest_ptr).unwrap() << 32) | u64::try_from(len).unwrap()
}

/// Given a merged `GuestPtrLen`, split out a `u32` pointer and length.
/// Performs the inverse of `merge_u64`. Takes the low `u32` bits as the length
/// then shifts the 32 high bits down and takes those as the pointer.
pub fn split_u64(u: GuestPtrLen) -> (GuestPtr, Len) {
    // It should be impossible to hit these verbose unwrap/panic conditions but it's more
    // conservative to define them than rely on an `as uX` cast that could silently truncate bits.
    (
        u32::try_from(u >> 32).unwrap(),
        u32::try_from(u & u64::try_from(u32::MAX).unwrap()).unwrap(),
    )
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let guest_ptr = 9000000;
        let len = 1000;

        let (out_guest_ptr, out_len) = split_u64(merge_u64(guest_ptr, len));

        assert_eq!(guest_ptr, out_guest_ptr,);
        assert_eq!(len, out_len,);
    }
}
