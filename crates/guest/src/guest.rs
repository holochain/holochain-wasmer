pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

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

#[macro_export]
/// Given a guest allocation pointer and a type that implements TryFrom<SerializedBytes>
/// - restore SerializedBytes from the guest pointer
/// - try to deserialize the given type from the restored SerializedBytes
/// - if the deserialization fails, short circuit (return early) with a WasmError
/// - if everything is Ok, return the restored data as a native rust type inside the guest
///
/// This works by assuming the host as already populated the guest pointer with the correct data
/// ahead of time.
macro_rules! host_args {
    ( $ptr:ident ) => {{
        use core::convert::TryInto;

        let bytes = match $crate::allocation::consume_bytes($ptr) {
            Ok(v) => v,
            Err(e) => {
                let sb = $crate::holochain_serialized_bytes::SerializedBytes::try_from(
                    $crate::result::WasmResult::Err($crate::result::WasmError::Memory),
                )
                .unwrap();
                return $crate::allocation::write_bytes(sb.bytes()).unwrap();
            }
        };

        match $crate::holochain_serialized_bytes::SerializedBytes::from(
            $crate::holochain_serialized_bytes::UnsafeBytes::from(bytes),
        )
        .try_into()
        {
            Ok(v) => v,
            Err(e) => {
                let sb = $crate::holochain_serialized_bytes::SerializedBytes::try_from(
                    $crate::result::WasmResult::Err($crate::result::WasmError::SerializedBytes(e)),
                )
                // Should be impossible to fail to serialize a simple enum variant.
                .unwrap();
                return $crate::allocation::write_bytes(sb.bytes()).unwrap();
            }
        }
    }};
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
pub fn host_call<'a, I: 'a, O>(
    f: unsafe extern "C" fn(GuestPtr) -> Len,
    payload: &'a I,
) -> Result<O, crate::WasmError>
where
    SerializedBytes: TryFrom<&'a I, Error = SerializedBytesError>,
    O: TryFrom<SerializedBytes, Error = holochain_serialized_bytes::SerializedBytesError>,
{
    let sb = SerializedBytes::try_from(payload)?;

    // Call the host function and receive the length of the serialized result.
    let input_guest_ptr = crate::allocation::write_bytes(sb.bytes())?;

    // This is unsafe because all host function calls in wasm are unsafe.
    let result_len: Len = unsafe { f(input_guest_ptr) };

    // Free the leaked bytes from the input to the host function.
    crate::allocation::__deallocate(input_guest_ptr);

    // Prepare a GuestPtr for the host to write into.
    let output_guest_ptr: GuestPtr = crate::allocation::__allocate(result_len);

    // Ask the host to populate the result allocation pointer with its result.
    unsafe { __import_data(output_guest_ptr) };

    // Deserialize the host bytes into the output type.
    Ok(O::try_from(SerializedBytes::from(UnsafeBytes::from(
        crate::allocation::consume_bytes(output_guest_ptr)?,
    )))?)
}

#[macro_export]
macro_rules! ret_err {
    ( $fail:expr ) => {{
        use std::convert::TryInto;
        let maybe_wasm_result_sb: std::result::Result<$crate::holochain_serialized_bytes::SerializedBytes, $crate::holochain_serialized_bytes::SerializedBytesError> =
            $crate::WasmResult::Err($crate::WasmError::Zome(String::from($fail))).try_into();
        match maybe_wasm_result_sb {
            std::result::Result::Ok(wasm_result_sb) => {
                return $crate::allocation::write_bytes(wasm_result_sb.bytes()).unwrap();
            },
            // we could end up down here if the fail string somehow fails to convert to SerializedBytes
            // for example it could be too big for messagepack or include invalid bytes
            std::result::Result::Err(e) => {
                return $crate::allocation::write_bytes($crate::holochain_serialized_bytes::SerializedBytes::try_from(
                    $crate::WasmResult::Err(
                        $crate::WasmError::Zome(
                            format!(
                                "errored while erroring (this should never happen): {:?}",
                                e
                            )
                        )
                    )
                ).unwrap().bytes()).unwrap();
            }
        };
    }};
}

#[macro_export]
macro_rules! ret {
    ( $e:expr) => {{
        use std::convert::TryInto;
        let maybe_sb: std::result::Result<$crate::holochain_serialized_bytes::SerializedBytes, $crate::holochain_serialized_bytes::SerializedBytesError> = ($e).try_into();
        match maybe_sb {
            Ok(sb) => {
                let maybe_wasm_result_sb: std::result::Result<$crate::holochain_serialized_bytes::SerializedBytes, $crate::holochain_serialized_bytes::SerializedBytesError> = $crate::WasmResult::Ok(sb).try_into();
                match maybe_wasm_result_sb {
                    std::result::Result::Ok(wasm_result_sb) => match $crate::allocation::write_bytes($crate::holochain_serialized_bytes::SerializedBytes::from(wasm_result_sb).bytes()) {
                        Ok(guest_ptr) => return guest_ptr,
                        Err(e) => $crate::ret_err!(e),
                    },
                    std::result::Result::Err(e) => $crate::ret_err!(e),
                };
            },
            std::result::Result::Err(e) => $crate::ret_err!(e),
        };
    }};
}

#[macro_export]
macro_rules! try_result {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => $crate::ret_err!(format!("{}: {:?}", $fail, e)),
        }
    }};
}
