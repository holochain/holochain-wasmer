pub extern crate holochain_serialized_bytes;

pub use holochain_wasmer_common::allocation;
pub use holochain_wasmer_common::*;

#[macro_export]
macro_rules! memory_externs {
    () => {
        extern "C" {
            // memory stuff
            fn __import_allocation(guest_allocation_ptr: RemotePtr, host_allocation_ptr: RemotePtr);
            fn __import_bytes(host_allocation_ptr: RemotePtr, guest_bytes_ptr: RemotePtr);
        }
    };
}
memory_externs!();

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( fn $func_name(guest_allocation_ptr: $crate::RemotePtr) -> $crate::RemotePtr; )*
        }
    };
}

/// given a pointer to an allocation on the host, copy the allocation into the guest and return the
/// guest's pointer to it
pub fn map_bytes(host_allocation_ptr: RemotePtr) -> AllocationPtr {
    let tmp_allocation: allocation::Allocation = [0, 0];
    let tmp_allocation_ptr: AllocationPtr = AllocationPtr::from(tmp_allocation);
    unsafe {
        __import_allocation(tmp_allocation_ptr.as_remote_ptr(), host_allocation_ptr);
    };
    // this allocation has the correct length but host bytes ptr
    let [_, len]: allocation::Allocation = allocation::Allocation::from(tmp_allocation_ptr);

    let guest_bytes_ptr: Ptr = allocation::allocate(len);
    unsafe {
        __import_bytes(host_allocation_ptr, guest_bytes_ptr);
    };
    let guest_allocation: allocation::Allocation = [guest_bytes_ptr, len];
    AllocationPtr::from(guest_allocation)
}

#[macro_export]
/// given a host allocation pointer and a type that implements TryFrom<JsonString>
/// - map bytes from the host into the guest
/// - restore a JsonString from the mapped bytes
/// - try to deserialize the given type from the restored JsonString
/// - if the deserialization fails, short circuit (return early) with a WasmError
/// - if everything is Ok, return the restored data as a native rust type inside the guest
macro_rules! host_args {
    ( $ptr:ident ) => {{
        use core::convert::TryInto;

        match $crate::holochain_serialized_bytes::SerializedBytes::from($crate::map_bytes($ptr))
            .try_into()
        {
            Ok(v) => v,
            Err(_) => {
                return $crate::AllocationPtr::from(
                    $crate::holochain_serialized_bytes::SerializedBytes::try_from(
                        $crate::result::WasmResult::Err(
                            $crate::result::WasmError::ArgumentDeserializationFailed,
                        ),
                    )
                    // should be impossible to fail to serialize a simple enum variant
                    .unwrap(),
                )
                .as_remote_ptr();
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
                let result_host_allocation_ptr: $crate::RemotePtr =
                    unsafe { $func_name($crate::AllocationPtr::from(sb).as_remote_ptr()) };
                let result_sb: $crate::holochain_serialized_bytes::SerializedBytes =
                    $crate::map_bytes(result_host_allocation_ptr).into();
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
                return $crate::AllocationPtr::from(wasm_result_sb).as_remote_ptr();
            },
            // we could end up down here if the fail string somehow fails to convert to SerializedBytes
            // for example it could be too big for messagepack or include invalid bytes
            std::result::Result::Err(e) => {
                return $crate::AllocationPtr::from($crate::holochain_serialized_bytes::SerializedBytes::try_from(
                    $crate::WasmResult::Err($crate::WasmError::Zome(String::from("errored while erroring (this should never happen)")))
                ).unwrap()).as_remote_ptr();
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
                    std::result::Result::Ok(wasm_result_sb) => return $crate::AllocationPtr::from($crate::holochain_serialized_bytes::SerializedBytes::from(wasm_result_sb)).as_remote_ptr(),
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
