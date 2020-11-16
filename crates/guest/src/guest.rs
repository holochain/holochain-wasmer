pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

#[macro_export]
macro_rules! memory_externs {
    () => {
        extern "C" {
            // memory stuff
            fn __import_data(guest_allocation_ptr: $crate::GuestPtr);
        }
    };
}

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( fn $func_name(guest_allocation_ptr: $crate::GuestPtr) -> $crate::Len; )*
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
/// given a guest allocation pointer and a type that implements TryFrom<SerializedBytes>
/// - restore SerializedBytes from the guest pointer
/// - try to deserialize the given type from the restored SerializedBytes
/// - if the deserialization fails, short circuit (return early) with a WasmError
/// - if everything is Ok, return the restored data as a native rust type inside the guest
/// this works by assuming the host as already populated the guest pointer with the correct data
/// ahead of time
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
                // should be impossible to fail to serialize a simple enum variant
                .unwrap();
                return $crate::allocation::write_bytes(sb.bytes()).unwrap();
            }
        }
    }};
}

#[macro_export]
macro_rules! host_call {
    ( $func_name:ident, $input:expr ) => {{
        use std::convert::TryInto;
        let maybe_sb: std::result::Result<
            $crate::holochain_serialized_bytes::SerializedBytes,
            $crate::holochain_serialized_bytes::SerializedBytesError,
        > = $input.try_into();
        match maybe_sb {
            std::result::Result::Ok(sb) => {
                // call the host function and receive the length of the serialized result
                let input_guest_ptr = $crate::allocation::write_bytes(sb.bytes()).unwrap();
                let result_len: $crate::Len = unsafe { $func_name(input_guest_ptr) };

                // free the leaked bytes from the input to the host function
                $crate::allocation::__deallocate(input_guest_ptr);

                // prepare a GuestPtr for the host to write into
                let guest_ptr: GuestPtr = $crate::allocation::__allocate(result_len);

                // ask the host to populate the result allocation pointer with its result
                unsafe {
                    __import_data(guest_ptr);
                };

                match $crate::allocation::consume_bytes(guest_ptr) {
                    Ok(result_bytes) => {
                        let result_sb = $crate::holochain_serialized_bytes::SerializedBytes::from(
                            $crate::holochain_serialized_bytes::UnsafeBytes::from(result_bytes),
                        );
                        result_sb.try_into()
                    }
                    Err(e) => unimplemented!(),
                }
            }
            std::result::Result::Err(e) => Err(e),
        }
    }};
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
