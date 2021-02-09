pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

use crate::allocation::consume_bytes;
use crate::allocation::write_bytes;

#[macro_export]
macro_rules! memory_externs {
    () => {
        extern "C" {
            // Memory stuff.
            fn __import_data(guest_allocation_ptr: $crate::GuestPtr);
        }
    };
}

memory_externs!();

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( pub fn $func_name(guest_allocation_ptr: $crate::GuestPtr) -> $crate::Len; )*
        }
    };
}

/// Receive arguments from the host.
/// The guest sets the type O that the host needs to match.
/// If deserialization fails then a `GuestPtr` to a `WasmError::Deserialize` is returned.
/// The guest should __immediately__ return an `Err` back to the host.
/// The `WasmError::Deserialize` enum contains the bytes that failed to deserialize so the host can
/// unambiguously provide debug information.
pub fn host_args<O>(ptr: GuestPtr) -> Result<O, GuestPtr>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = consume_bytes(ptr);

    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(return_err_ptr(WasmError::Deserialize(bytes)))
        }
    }
}

/// Given an `GuestPtr` -> `Len` extern that we expect the host to provide:
/// - Serialize the payload by reference
/// - Write the bytes into a new allocation
/// - Call the host function and pass it the pointer to our allocation full of serialized data
/// - Deallocate the serialized bytes when the host function completes
/// - Allocate empty bytes of the length that the host tells us the result is
/// - Ask the host to write the result into the allocated empty bytes
/// - Deserialize and deallocate whatever bytes the host has written into the result allocation
/// - Return a Result of the deserialized output type O
pub fn host_call<I, O>(
    f: unsafe extern "C" fn(GuestPtr) -> Len,
    input: I,
) -> Result<O, crate::WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Call the host function and receive the length of the serialized result.
    let input_guest_ptr =
        crate::allocation::write_bytes(&holochain_serialized_bytes::encode(&input)?);

    // This is unsafe because all host function calls in wasm are unsafe.
    let result_len: Len = unsafe { f(input_guest_ptr) };

    // Free the leaked bytes from the input to the host function.
    crate::allocation::__deallocate(input_guest_ptr);

    // Prepare a GuestPtr for the host to write into.
    let output_guest_ptr: GuestPtr = crate::allocation::__allocate(result_len);

    // Ask the host to populate the result allocation pointer with its result.
    unsafe { __import_data(output_guest_ptr) };

    // Deserialize the host bytes into the output type.
    let bytes: Vec<u8> = crate::allocation::consume_bytes(output_guest_ptr);
    match holochain_serialized_bytes::decode::<Vec<u8>, Result<O, WasmError>>(&bytes) {
        Ok(output) => Ok(output?),
        Err(e) => {
            tracing::error!(output_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(WasmError::Deserialize(bytes))
        }
    }
}

/// Convert any serializable value into a GuestPtr that can be returned to the host.
/// The host is expected to know how to consume and deserialize it.
pub fn return_ptr<R>(return_value: R) -> GuestPtr
where
    R: Serialize + std::fmt::Debug,
{
    match holochain_serialized_bytes::encode::<Result<R, WasmError>>(&Ok(return_value)) {
        Ok(bytes) => write_bytes(&bytes),
        Err(e) => return_err_ptr(WasmError::Serialize(e)),
    }
}

/// Convert an Into<String> into a generic `Err(WasmError::Guest)` as a `GuestPtr` returned.
pub fn return_err_ptr(wasm_error: WasmError) -> GuestPtr {
    match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(wasm_error)) {
        Ok(bytes) => write_bytes(&bytes),
        Err(e) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
            WasmError::Serialize(e),
        )) {
            Ok(bytes) => write_bytes(&bytes),
            // At this point we've errored while erroring
            Err(_) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                WasmError::ErrorWhileError,
            )) {
                Ok(bytes) => write_bytes(&bytes),
                // At this point we failed to serialize a unit struct so IDK ¯\_(ツ)_/¯
                Err(_) => unreachable!(),
            },
        },
    }
}

#[macro_export]
/// A simple macro to wrap return_err_ptr in an analogy to the native rust `?`.
macro_rules! try_ptr {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return return_err_ptr(WasmError::Guest(format!("{}: {:?}", $fail, e))),
        }
    }};
}
