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

pub fn merge_u64(a: u64, b: u64) -> Result<u128, WasmError> {
    Ok(
        (u128::try_from(a).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?
            << (std::mem::size_of::<u64>() * 8))
            | u128::try_from(b).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
    )
}

pub fn merge_u32(a: u32, b: u32) -> Result<u64, WasmError> {
    Ok(
        (u64::try_from(a).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?
            << (std::mem::size_of::<u32>() * 8))
            | u64::try_from(b).map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
    )
}

/// Given 2x `u32`, return a `DoubleUSize` merged.
/// Works via a simple bitwise shift to move the pointer to high bits then OR
/// the length into the low bits.
pub fn merge_usize(a: usize, b: usize) -> Result<DoubleUSize, WasmError> {
    #[cfg(target_pointer_width = "64")]
    return merge_u64(a as u64, b as u64);
    #[cfg(target_pointer_width = "32")]
    return merge_u32(a as u32, b as u32);
}

pub fn split_u128(u: u128) -> Result<(u64, u64), WasmError> {
    Ok((
        u64::try_from(u >> (std::mem::size_of::<u64>() * 8))
            .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
        u64::try_from(u & (u64::MAX as u128))
            .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
    ))
}

pub fn split_u64(u: u64) -> Result<(u32, u32), WasmError> {
    Ok((
        u32::try_from(u >> (std::mem::size_of::<u32>() * 8))
            .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
        u32::try_from(u & (u32::MAX as u64))
            .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
    ))
}

/// Given 2x merged `usize`, split out two `usize`.
/// Performs the inverse of `merge_usize`.
pub fn split_usize(u: DoubleUSize) -> Result<(usize, usize), WasmError> {
    #[cfg(target_pointer_width = "64")]
    return split_u128(u).map(|(a, b)| (a as usize, b as usize));
    #[cfg(target_pointer_width = "32")]
    return split_u64(u as u64).map(|(a, b)| (a as usize, b as usize));
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn _round_trip_u32(a: u32, b: u32) {
        let (out_a, out_b) = split_u64(merge_u32(a, b).unwrap()).unwrap();

        assert_eq!(a, out_a);
        assert_eq!(b, out_b);
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn round_trip_u32(a: u32, b: u32) {
        _round_trip_u32(a, b);
    }

    #[test]
    fn some_round_trip_u32() {
        _round_trip_u32(u32::MAX, u32::MAX);
    }

    fn _round_trip_u64(a: u64, b: u64) {
        let (out_a, out_b) = split_u128(merge_u64(a, b).unwrap()).unwrap();

        assert_eq!(a, out_a);
        assert_eq!(b, out_b);
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn round_trip_u64(a: u64, b: u64) {
        _round_trip_u64(a, b);
    }

    #[test]
    fn some_round_trip_u64() {
        _round_trip_u64(u64::MAX, u64::MAX);
    }

    fn _round_trip_usize(a: usize, b: usize) {
        let (out_a, out_b) = split_usize(merge_usize(a, b).unwrap()).unwrap();

        assert_eq!(a, out_a,);
        assert_eq!(b, out_b,);
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn round_trip_usize(a: usize, b: usize) {
        _round_trip_usize(a, b);
    }

    #[test]
    fn some_round_trip_usize() {
        _round_trip_usize(usize::MAX, usize::MAX);
    }
}
