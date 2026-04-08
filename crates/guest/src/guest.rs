//! Guest-side runtime for wasm modules running under
//! [`holochain_wasmer_host`](https://docs.rs/holochain_wasmer_host).
//! Provides the macros and helper functions a wasm zome (or similar
//! wasm-as-plugin) needs to talk to a Holochain host: declaring extern
//! host functions, decoding host inputs, calling back into the host,
//! and returning serializable values across the host↔guest boundary.
//!
//! Most users do not depend on this crate directly — the
//! [Holochain HDK](https://docs.rs/hdk) hides the host/guest plumbing
//! entirely and zome authors should reach for it instead. The contents
//! below are for HDK maintainers, for the macros' implementations, and
//! for anyone embedding Holochain wasms outside of an HDK context.
//!
//! # Conceptual model
//!
//! Wasm has only four primitive types (`i32`, `i64`, `f32`, `f64`) and
//! a single shared linear memory. There are no strings, sequences,
//! structs or other complex types in the wasm ABI itself. To pass
//! anything richer between host and guest, both sides agree on a
//! byte-level protocol: the sender writes serialized bytes into the
//! shared linear memory, the receiver reads them out at a known
//! pointer and length.
//!
//! This crate's job is to make that protocol invisible at the call
//! site. You write what looks like a Rust function with serializable
//! inputs and outputs, and the macros and helpers below take care of
//! the leak-and-pointer dance underneath.
//!
//! Constraints to keep in mind:
//!
//! - The host has full access to guest memory; the guest has none of
//!   the host's. The only way the guest can read host data is to call
//!   an imported host function and have the host write the response
//!   into shared linear memory.
//! - When the host calls into the guest, the host cannot call back
//!   into the same guest instance from inside that call. The guest
//!   call must complete (or trap) before the host can re-enter.
//! - Wasm linear memory pages can be added but never removed. A guest
//!   that allocates aggressively will hold that memory for its
//!   lifetime, so be conservative about copying large payloads.
//! - The serialization format must round-trip cleanly (we use
//!   `holochain_serialized_bytes`, which wraps messagepack). It is the
//!   caller's responsibility to ensure types implement `Serialize` /
//!   `DeserializeOwned` consistently on both sides.
//!
//! All code samples in the rest of this documentation are
//! `// ignore`-marked rustdoc blocks. They are not run as doctests
//! because guest code is meant to be compiled to the
//! `wasm32-unknown-unknown` target with the `__hc__allocate_1` /
//! `__hc__deallocate_1` exports and the host's imported functions
//! linked in — none of which are present when rustdoc compiles a
//! doctest as a host-side binary. Treat them as the canonical shape
//! to copy from rather than as runnable snippets; the
//! [`test-crates/wasms/`](https://github.com/holochain/holochain-wasmer/tree/main/test-crates/wasms)
//! directory in the repository contains the same patterns inside
//! real wasm crates that CI exercises against a real host on every
//! PR.
//!
//! # Declaring host functions you want to call
//!
//! The host exposes a set of imported functions to each guest
//! instance. Use the [`host_externs!`] macro to declare the ones you
//! intend to call. The macro takes pairs of `name:version` and
//! generates `extern "C"` declarations of the form
//! `__hc__<name>_<version>`:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! host_externs!(test_process_string:2, debug:1);
//! // Generates:
//! //   extern "C" fn __hc__test_process_string_2(ptr: usize, len: usize) -> DoubleUSize;
//! //   extern "C" fn __hc__debug_1(ptr: usize, len: usize) -> DoubleUSize;
//! ```
//!
//! The version suffix lets the host evolve a function's signature
//! without breaking older guests; new guests opt in to the new
//! version by bumping the literal.
//!
//! # Writing functions the host can call
//!
//! Every function the host can call into must have the signature
//! `extern "C" fn(guest_ptr: usize, len: usize) -> DoubleUSize`. The
//! `#[no_mangle]` attribute keeps the symbol name stable so the host
//! can look it up by string:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! #[no_mangle]
//! pub extern "C" fn process_string(guest_ptr: usize, len: usize) -> DoubleUSize {
//!     let input: String = match host_args(guest_ptr, len) {
//!         Ok(v) => v,
//!         Err(err_ptr) => return err_ptr,
//!     };
//!     return_ptr(format!("guest: {}", input))
//! }
//! ```
//!
//! The `(guest_ptr, len) -> DoubleUSize` shape is forced by what
//! stable Rust's `extern "C"` ABI can express on `wasm32-unknown-unknown`.
//! Multi-value returns would let us return a `(ptr, len)` tuple
//! directly, but they require nightly's `extern "wasm"`. Instead we
//! pack both `u32`s into a single `u64` (a `DoubleUSize` on a 32-bit
//! target) and split it again on the host side.
//!
//! # Receiving input with [`host_args`]
//!
//! [`host_args`] takes the `(guest_ptr, len)` pair the host passed in
//! and tries to deserialize it into your input type:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! #[no_mangle]
//! pub extern "C" fn foo(guest_ptr: usize, len: usize) -> DoubleUSize {
//!     let input: MyInputType = match host_args(guest_ptr, len) {
//!         Ok(v) => v,
//!         Err(err_ptr) => return err_ptr,
//!     };
//!     // ... use input ...
//!     # return_ptr(())
//! }
//! # #[derive(serde::Deserialize, serde::Serialize, Debug)]
//! # struct MyInputType;
//! ```
//!
//! If deserialization fails, [`host_args`] returns
//! `Err(DoubleUSize)` — a pointer to a serialized [`WasmError`] that
//! the host knows how to read. **The guest must immediately return
//! that pointer**; trying to recover or call further host functions
//! after a deserialization failure leaves the guest in an
//! inconsistent state and risks corrupting memory.
//!
//! # Calling host functions with [`host_call`]
//!
//! Unlike [`host_args`], [`host_call`] returns a native Rust
//! [`Result`], so it works anywhere — including outside of an extern
//! function. Pass it the extern declared by [`host_externs!`] and a
//! serializable input; it deserializes the host's response into the
//! type you ask for:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! host_externs!(test_process_string:2);
//!
//! fn call_host() -> Result<String, WasmError> {
//!     let input = String::from("hello");
//!     let output: String = host_call(__hc__test_process_string_2, &input)?;
//!     Ok(output)
//! }
//! ```
//!
//! Inside an `extern "C"` guest function — where the return type is
//! `DoubleUSize`, not `Result` — wrap the call with the [`try_ptr!`]
//! macro to get `?`-style early return:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! host_externs!(test_process_string:2);
//!
//! #[no_mangle]
//! pub extern "C" fn process(_: usize, _: usize) -> DoubleUSize {
//!     let input = String::from("hello");
//!     let output: String = try_ptr!(
//!         host_call(__hc__test_process_string_2, &input),
//!         "test_process_string_2 failed"
//!     );
//!     return_ptr(output)
//! }
//! ```
//!
//! # Returning to the host with [`return_ptr`] / [`return_err_ptr`]
//!
//! Inside an extern, the return value the host receives must be a
//! `DoubleUSize`. Use [`return_ptr`] for any serializable success
//! value and [`return_err_ptr`] for a [`WasmError`]:
//!
//! ```ignore
//! use holochain_wasmer_guest::*;
//!
//! #[no_mangle]
//! pub extern "C" fn ok_example(_: usize, _: usize) -> DoubleUSize {
//!     return_ptr("hello from the guest".to_string())
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn err_example(_: usize, _: usize) -> DoubleUSize {
//!     return_err_ptr(wasm_error!(WasmErrorInner::Guest("nope".into())))
//! }
//! ```
//!
//! The host treats every guest return value as an "outer" `Result`:
//! `Ok` means the guest completed normally and any inner success or
//! domain error is encoded in the value, while `Err` means the
//! host↔guest interface itself failed (deserialization, missing
//! extern, etc.) and the host should treat the instance as suspect.
//!
//! # Worked examples
//!
//! See [`test-crates/wasms/wasm_core/src/wasm.rs`](https://github.com/holochain/holochain-wasmer/blob/main/test-crates/wasms/wasm_core/src/wasm.rs)
//! in the `holochain-wasmer` repository for a complete test wasm that
//! exercises every macro and helper in this crate against a real host
//! built from `holochain_wasmer_host`.

pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

use crate::allocation::consume_bytes;
use crate::allocation::write_bytes;

pub use paste::paste;

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident:$version:literal ),* ) => {
        $crate::paste! {
            #[no_mangle]
            extern "C" {
                $( pub fn [<__hc__ $func_name _ $version>](guest_allocation_ptr: usize, len: usize) -> $crate::DoubleUSize; )*
            }
        }
    };
}

/// Receive arguments from the host.
/// The guest sets the type `O` that the host needs to match.
/// If deserialization fails then a `GuestPtr` to a `WasmError::Deserialize` is returned.
/// The guest should __immediately__ return an `Err` back to the host.
/// The `WasmError::Deserialize` enum contains the bytes that failed to deserialize so the host can
/// unambiguously provide debug information.
#[inline(always)]
pub fn host_args<O>(ptr: usize, len: usize) -> Result<O, DoubleUSize>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = consume_bytes(ptr, len);
    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(return_err_ptr(wasm_error!(WasmErrorInner::Deserialize(
                bytes
            ))))
        }
    }
}

/// Given an extern that we expect the host to provide:
/// - Serialize the payload by reference
/// - Write the bytes into a new allocation on the guest side
/// - Call the host function and pass it the pointer and length to our leaked serialized data
/// - The host will consume and deallocate the bytes
/// - Deserialize whatever bytes we can import from the host after calling the host function
/// - Return a `Result` of the deserialized output type `O`
#[inline(always)]
#[allow(improper_ctypes_definitions)]
pub fn host_call<I, O>(
    f: unsafe extern "C" fn(usize, usize) -> DoubleUSize,
    input: I,
) -> Result<O, crate::WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Call the host function and receive the length of the serialized result.
    let mut input_bytes = holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e))?;
    input_bytes.shrink_to_fit();
    if input_bytes.capacity() != input_bytes.len() {
        tracing::warn!("Capacity should equal length, dealloc will fail");
    }
    debug_assert!(
        input_bytes.capacity() == input_bytes.len(),
        "Capacity should equal length, dealloc would fail"
    );
    let input_len: usize = input_bytes.len();
    let input_guest_ptr = crate::allocation::write_bytes(input_bytes);

    let (output_guest_ptr, output_len): (usize, usize) = split_usize(unsafe {
        // This is unsafe because all host function calls in wasm are unsafe.
        // The host will call `__hc__deallocate_1` for us to free the leaked bytes from the input.
        f(input_guest_ptr, input_len)
    })?;

    // Deserialize the host bytes into the output type.
    let bytes = crate::allocation::consume_bytes(output_guest_ptr, output_len);
    match holochain_serialized_bytes::decode::<[u8], Result<O, WasmError>>(&bytes) {
        Ok(output) => Ok(output?),
        Err(e) => {
            tracing::error!(output_type = std::any::type_name::<O>(), ?bytes, "{}", e);
            Err(wasm_error!(WasmErrorInner::Deserialize(bytes)))
        }
    }
}

/// Convert any serializable value into a `GuestPtr` that can be returned to the host.
/// The host is expected to know how to consume and deserialize it.
#[inline(always)]
pub fn return_ptr<R>(return_value: R) -> DoubleUSize
where
    R: Serialize + std::fmt::Debug,
{
    match holochain_serialized_bytes::encode::<Result<R, WasmError>>(&Ok(return_value)) {
        Ok(mut bytes) => {
            let len: usize = bytes.len();
            bytes.shrink_to_fit();
            if bytes.capacity() != bytes.len() {
                tracing::warn!("Capacity should equal length, dealloc will fail");
            }
            debug_assert!(
                bytes.capacity() == bytes.len(),
                "Capacity should equal length, dealloc would fail"
            );
            merge_usize(write_bytes(bytes), len).unwrap_or_else(return_err_ptr)
        }
        Err(e) => return_err_ptr(wasm_error!(WasmErrorInner::Serialize(e))),
    }
}

/// Convert a `WasmError` to a `GuestPtrLen` as best we can. This is not
/// necessarily straightforward as the serialization process can error recursively.
/// In the worst case we can't even serialize an enum variant, in which case we panic.
/// The casts from `usize` to `u32` are safe as long as the guest code is compiled
/// for `wasm32-unknown-unknown` target.
#[inline(always)]
pub fn return_err_ptr(wasm_error: WasmError) -> DoubleUSize {
    let mut bytes =
        match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(wasm_error)) {
            Ok(bytes) => bytes,
            Err(e) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                wasm_error!(WasmErrorInner::Serialize(e)),
            )) {
                Ok(bytes) => bytes,
                // At this point we've errored while erroring
                Err(_) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                    wasm_error!(WasmErrorInner::ErrorWhileError),
                )) {
                    Ok(bytes) => bytes,
                    // At this point we failed to serialize a unit variant so IDK ¯\_(ツ)_/¯
                    Err(_) => panic!("Failed to error"),
                },
            },
        };
    bytes.shrink_to_fit();
    if bytes.capacity() != bytes.len() {
        tracing::warn!("Capacity should equal length, dealloc will fail");
    }
    debug_assert!(
        bytes.capacity() == bytes.len(),
        "Capacity should equal length, dealloc would fail"
    );
    let len = bytes.len();
    merge_usize(write_bytes(bytes), len).expect("Failed to build return value")
}

/// A simple macro to wrap `return_err_ptr` in an analogy to the native rust `?`.
#[macro_export]
macro_rules! try_ptr {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return return_err_ptr(wasm_error!("{}: {:?}", $fail, e)),
        }
    }};
}
