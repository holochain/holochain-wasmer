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

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( pub fn $func_name(guest_allocation_ptr: $crate::GuestPtr) -> $crate::Len; )*
        }
    };
}

#[macro_export]
macro_rules! holochain_externs {
    () => {
        $crate::memory_externs!();
        $crate::host_externs!(
            __debug,
            __hash_entry,
            __unreachable,
            __verify_signature,
            __sign,
            __decrypt,
            __encrypt,
            __zome_info,
            __property,
            __random_bytes,
            __show_env,
            __sys_time,
            __agent_info,
            __capability_claims,
            __capability_grants,
            __capability_info,
            __get,
            __get_details,
            __get_links,
            __get_link_details,
            __get_agent_activity,
            __query,
            __call_remote,
            __call,
            __create,
            __emit_signal,
            __remote_signal,
            __create_link,
            __delete_link,
            __update,
            __delete,
            __schedule
        );
    };
}

holochain_externs!();

pub fn host_args<O>(ptr: GuestPtr) -> Result<O, GuestPtr>
where
    O: serde::de::DeserializeOwned,
{
    let bytes = consume_bytes(ptr);

    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            let deserialize_error: Result<(), WasmError> = Err(WasmError::Serialization(format!(
                "{}, {:?}",
                e.to_string(),
                &bytes
            )));
            return Err(return_ptr(deserialize_error));
        }
    }
}

/// Given an extern that we expect the host to provide, that takes a GuestPtr and returns a Len:
/// - Serialize the payload by reference, according to its SerializedBytes implementation
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
    WasmIO<I>: From<I>,
    I: serde::Serialize,
    O: serde::de::DeserializeOwned,
{
    // Call the host function and receive the length of the serialized result.
    let input_guest_ptr = crate::allocation::write_bytes(&WasmIO::from(input).try_to_bytes()?);

    // This is unsafe because all host function calls in wasm are unsafe.
    let result_len: Len = unsafe { f(input_guest_ptr) };

    // Free the leaked bytes from the input to the host function.
    crate::allocation::__deallocate(input_guest_ptr);

    // Prepare a GuestPtr for the host to write into.
    let output_guest_ptr: GuestPtr = crate::allocation::__allocate(result_len);

    // Ask the host to populate the result allocation pointer with its result.
    unsafe { __import_data(output_guest_ptr) };

    // Deserialize the host bytes into the output type.
    Ok(holochain_serialized_bytes::decode(
        &crate::allocation::consume_bytes(output_guest_ptr),
    )?)
}

pub fn return_ptr<R>(return_value: R) -> GuestPtr
where
    WasmIO<Result<R, WasmError>>: From<Result<R, WasmError>>,
    R: Serialize,
{
    match WasmIO::from(Ok(return_value)).try_to_bytes() {
        Ok(bytes) => write_bytes(&bytes),
        Err(e) => {
            let serialization_error: Result<(), WasmError> =
                Err(WasmError::Serialization(e.to_string()));
            return_ptr::<Result<(), WasmError>>(serialization_error)
        }
    }
}

pub fn return_err_ptr<S>(error_message: S) -> GuestPtr
where
    String: From<S>,
{
    let error: Result<(), WasmError> = Err(WasmError::Zome(error_message.into()));
    return_ptr::<Result<(), WasmError>>(error)
}

#[macro_export]
macro_rules! try_ptr {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return return_err_ptr(format!("{}: {:?}", $fail, e)),
        }
    }};
}
