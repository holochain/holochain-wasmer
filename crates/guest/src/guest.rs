pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

use crate::allocation::consume_bytes;
use crate::allocation::write_bytes;

extern "C" {
    fn __import_data() -> u64;
}

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( pub fn $func_name(guest_allocation_ptr: $crate::GuestPtr, len: $crate::Len); )*
        }
    };
}

/// Receive arguments from the host.
/// The guest sets the type O that the host needs to match.
/// If deserialization fails then a `GuestPtr` to a `WasmError::Deserialize` is returned.
/// The guest should __immediately__ return an `Err` back to the host.
/// The `WasmError::Deserialize` enum contains the bytes that failed to deserialize so the host can
/// unambiguously provide debug information.
#[inline(always)]
pub fn host_args<O>(ptr: GuestPtr, len: Len) -> Result<O, GuestPtrLen>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = consume_bytes(ptr, len);
    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(return_err_ptr(WasmError::Deserialize(bytes)))
        }
    }
}

/// Given an extern that we expect the host to provide:
/// - Serialize the payload by reference
/// - Write the bytes into a new allocation on the guest side
/// - Call the host function and pass it the pointer and length to our leaked serialized data
/// - The host will consume and deallocate the bytes
/// - Deserialize whatever bytes we can import from the host after calling the host function
/// - Return a Result of the deserialized output type O
#[inline(always)]
pub fn host_call<I, O>(
    f: unsafe extern "C" fn(GuestPtr, Len),
    input: I,
) -> Result<O, crate::WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Call the host function and receive the length of the serialized result.
    let input_bytes = holochain_serialized_bytes::encode(&input)?;
    let input_len = input_bytes.len();
    let input_guest_ptr = crate::allocation::write_bytes(input_bytes);

    let (output_guest_ptr, output_len): (GuestPtr, Len) = unsafe {
        // This is unsafe because all host function calls in wasm are unsafe.
        // The host will call __deallocate for us to free the leaked bytes from the input.
        f(input_guest_ptr, input_len as Len);
        // Get the guest pointer to the result of calling the above function on the host.
        split_u64(__import_data())
    };

    // Deserialize the host bytes into the output type.
    let bytes: Vec<u8> = crate::allocation::consume_bytes(output_guest_ptr, output_len);
    match holochain_serialized_bytes::decode::<Vec<u8>, Result<O, WasmError>>(&bytes) {
        Ok(output) => Ok(output?),
        Err(e) => {
            tracing::error!(output_type = std::any::type_name::<O>(), ?bytes, "{}", e);
            Err(WasmError::Deserialize(bytes))
        }
    }
}

/// Convert any serializable value into a GuestPtr that can be returned to the host.
/// The host is expected to know how to consume and deserialize it.
#[inline(always)]
pub fn return_ptr<R>(return_value: R) -> GuestPtrLen
where
    R: Serialize + std::fmt::Debug,
{
    match holochain_serialized_bytes::encode::<Result<R, WasmError>>(&Ok(return_value)) {
        Ok(bytes) => {
            let len = bytes.len();
            merge_u64(write_bytes(bytes), len as Len)
        }
        Err(e) => return_err_ptr(WasmError::Serialize(e)),
    }
}

/// Convert a WasmError to a GuestPtrLen as best we can.
#[inline(always)]
pub fn return_err_ptr(wasm_error: WasmError) -> GuestPtrLen {
    match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(wasm_error)) {
        Ok(bytes) => {
            let len = bytes.len();
            merge_u64(write_bytes(bytes), len as Len)
        }
        Err(e) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
            WasmError::Serialize(e),
        )) {
            Ok(bytes) => {
                let len = bytes.len();
                merge_u64(write_bytes(bytes), len as Len)
            }
            // At this point we've errored while erroring
            Err(_) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                WasmError::ErrorWhileError,
            )) {
                Ok(bytes) => {
                    let len = bytes.len();
                    merge_u64(write_bytes(bytes), len as Len)
                }
                // At this point we failed to serialize a unit struct so IDK ¯\_(ツ)_/¯
                Err(_) => unreachable!(),
            },
        },
    }
}

/// A simple macro to wrap return_err_ptr in an analogy to the native rust `?`.
#[macro_export]
macro_rules! try_ptr {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return return_err_ptr(WasmError::Guest(format!("{}: {:?}", $fail, e))),
        }
    }};
}
