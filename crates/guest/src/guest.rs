pub mod allocation;

pub extern crate holochain_serialized_bytes;

pub use crate::allocation::AllocationPtr;
pub use holochain_wasmer_common::slice;
pub use holochain_wasmer_common::*;

#[no_mangle]
pub extern "C" fn __allocation_ptr_uninitialized(len: Len) -> AllocationPtr {
    AllocationPtr::from(SerializedBytes::from(UnsafeBytes::from(vec![0; len as _])))
}

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
            __globals,
            __call,
            __capability,
            __commit_entry,
            __decrypt,
            __encrypt,
            __show_env,
            __property,
            __query,
            __remove_link,
            __send,
            __sign,
            __schedule,
            __update_entry,
            __emit_signal,
            __remove_entry,
            __link_entries,
            __keystore,
            __get_links,
            __get_entry,
            __entry_type_properties,
            __entry_address,
            __sys_time,
            __debug
        );
    };
}

memory_externs!();

#[no_mangle]
/// when we return an AllocationPtr(serialized_bytes).as_guest_ptr() to the host there is still a
/// bunch of SerializedBytes sitting in the wasm memory
/// the host needs to read these bytes out of memory to get the return value from the guest but
/// the host then needs to tell the guest that previously leaked memory can be freed
/// this function allows the host to notify the guest that it is finished with a GuestPtr
pub extern "C" fn __deallocate_guest_allocation(guest_ptr: GuestPtr) {
    // rehydrating and dropping the SerializedBytes is enough for allocations to be cleaned up
    let _: SerializedBytes = AllocationPtr::from_guest_ptr(guest_ptr).into();
}

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

        let ptr = $crate::allocation::AllocationPtr::from_guest_ptr($ptr);

        match $crate::holochain_serialized_bytes::SerializedBytes::from(ptr).try_into() {
            Ok(v) => v,
            Err(e) => {
                return $crate::allocation::AllocationPtr::from(
                    $crate::holochain_serialized_bytes::SerializedBytes::try_from(
                        $crate::result::WasmResult::Err(
                            $crate::result::WasmError::SerializedBytes(e),
                        ),
                    )
                    // should be impossible to fail to serialize a simple enum variant
                    .unwrap(),
                )
                .as_guest_ptr();
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
                // prepare an input allocation pointer so the host can read sb out of the guest
                let input_allocation_ptr: $crate::allocation::AllocationPtr = sb.into();

                // call the host function and receive the length of the serialized result
                let result_len: $crate::Len =
                    unsafe { $func_name(input_allocation_ptr.as_guest_ptr()) };

                // drop the input data here on the guest side so it doesn't leak
                let _: $crate::holochain_serialized_bytes::SerializedBytes =
                    input_allocation_ptr.into();

                // prepare a new allocation pointer on the guest side to store the result
                let result_allocation_ptr: $crate::allocation::AllocationPtr =
                    $crate::__allocation_ptr_uninitialized(result_len);

                // ask the host to populate the result allocation pointer with its result
                unsafe {
                    __import_data(result_allocation_ptr.as_guest_ptr());
                };

                // pull the imported data into SerializedBytes
                let result_sb: $crate::holochain_serialized_bytes::SerializedBytes =
                    result_allocation_ptr.into();

                // deserialize
                result_sb.try_into()
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
                return $crate::allocation::AllocationPtr::from(wasm_result_sb).as_guest_ptr();
            },
            // we could end up down here if the fail string somehow fails to convert to SerializedBytes
            // for example it could be too big for messagepack or include invalid bytes
            std::result::Result::Err(e) => {
                return $crate::allocation::AllocationPtr::from($crate::holochain_serialized_bytes::SerializedBytes::try_from(
                    $crate::WasmResult::Err($crate::WasmError::Zome(String::from("errored while erroring (this should never happen)")))
                ).unwrap()).as_guest_ptr();
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
                    std::result::Result::Ok(wasm_result_sb) => return $crate::allocation::AllocationPtr::from($crate::holochain_serialized_bytes::SerializedBytes::from(wasm_result_sb)).as_guest_ptr(),
                    std::result::Result::Err(e) => ret_err!(e),
                };
            },
            std::result::Result::Err(e) => ret_err!(e),
        };
    }};
}

#[macro_export]
macro_rules! try_result {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(_) => $crate::ret_err!($fail),
        }
    }};
}
