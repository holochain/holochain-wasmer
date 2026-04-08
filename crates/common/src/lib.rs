//! Types and helpers shared between
//! [`holochain_wasmer_host`](https://docs.rs/holochain_wasmer_host) and
//! `holochain_wasmer_guest`: the [`WasmError`] / [`WasmErrorInner`]
//! error model, the [`wasm_error!`] convenience macro, and the small
//! numeric helpers ([`merge_usize`] / [`split_usize`] etc) used to
//! pack pointer/length pairs across the host↔guest boundary.
//!
//! # Cargo features
//!
//! - **`error-as-host`** — when constructing a [`WasmError`] from a
//!   bare `String`, classify it as [`WasmErrorInner::Host`] rather
//!   than [`WasmErrorInner::Guest`]. Hosts that build error strings
//!   should enable this; guests should leave it off. The host crate
//!   enables it via its own `error-as-host` feature.

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
    Ok((u128::from(a) << (std::mem::size_of::<u64>() * 8)) | u128::from(b))
}

pub fn merge_u32(a: u32, b: u32) -> Result<u64, WasmError> {
    Ok((u64::from(a) << (std::mem::size_of::<u32>() * 8)) | u64::from(b))
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
    return split_u64(u).map(|(a, b)| (a as usize, b as usize));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_u32() {
        for (a, b) in [
            (0, 0),
            (0, 1),
            (1, 0),
            (1, u32::MAX),
            (u32::MAX, 1),
            (u32::MAX, u32::MAX),
            (0xdead_beef, 0xcafe_d00d),
        ] {
            let (out_a, out_b) = split_u64(merge_u32(a, b).unwrap()).unwrap();
            assert_eq!(a, out_a);
            assert_eq!(b, out_b);
        }
    }

    #[test]
    fn round_trip_u64() {
        for (a, b) in [
            (0, 0),
            (0, 1),
            (1, 0),
            (1, u64::MAX),
            (u64::MAX, 1),
            (u64::MAX, u64::MAX),
            (0xdead_beef_dead_beef, 0xcafe_d00d_cafe_d00d),
        ] {
            let (out_a, out_b) = split_u128(merge_u64(a, b).unwrap()).unwrap();
            assert_eq!(a, out_a);
            assert_eq!(b, out_b);
        }
    }

    #[test]
    fn round_trip_usize() {
        for (a, b) in [
            (0, 0),
            (0, 1),
            (1, 0),
            (1, usize::MAX),
            (usize::MAX, 1),
            (usize::MAX, usize::MAX),
        ] {
            let (out_a, out_b) = split_usize(merge_usize(a, b).unwrap()).unwrap();
            assert_eq!(a, out_a);
            assert_eq!(b, out_b);
        }
    }
}
