pub mod result;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;
pub use serde_bytes;

/// Something like `usize` for wasm.
/// Wasm has a memory limit of 4GB so offsets and lengths fit in `u32`.
///
/// When rust compiles to the `wasm32-unknown-unknown` target, `usize` will be `u32` in
/// wasm, but the host could interpret `usize` as either `u32` or `u64`. For that reason
/// we specify `u32` everywhere we need the host and the guest to have a shared agreement
/// on the size of an offset/length, or the host will not be able to directly
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

#[cfg(target_pointer_width = "16")]
pub type DoubleUSize = u32;

#[cfg(target_pointer_width = "32")]
pub type DoubleUSize = u64;

#[cfg(target_pointer_width = "64")]
pub type DoubleUSize = u128;

const SPLIT_MASK: DoubleUSize = usize::MAX as DoubleUSize;

/// Given 2x `u32`, return a `DoubleUSize` merged.
/// Works via a simple bitwise shift to move the pointer to high bits then OR
/// the length into the low bits.
pub fn merge_usize(a: usize, b: usize) -> Result<DoubleUSize, WasmError> {
    Ok(
        (DoubleUSize::try_from(a).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?
            << (std::mem::size_of::<usize>() * 8))
            | DoubleUSize::try_from(b).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
    )
}

/// Given 2x merged `usize`, split out two `usize`.
/// Performs the inverse of `merge_usize`.
pub fn split_usize(u: DoubleUSize) -> Result<(usize, usize), WasmError> {
    Ok((
        usize::try_from(u >> (std::mem::size_of::<usize>() * 8))
            .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
        usize::try_from(u & SPLIT_MASK).unwrap(),
    ))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test_fuzz::test_fuzz]
    fn round_trip_ptrlen(guest_ptr: usize, len: usize) {
        let (out_guest_ptr, out_len) = split_usize(merge_usize(guest_ptr, len));

        assert_eq!(guest_ptr, out_guest_ptr,);
        assert_eq!(len, out_len,);
    }
}
