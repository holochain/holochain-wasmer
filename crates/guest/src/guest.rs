pub use holochain_wasmer_common::allocation;
pub use holochain_wasmer_common::bytes;
pub use holochain_wasmer_common::error::Error;
pub use holochain_wasmer_common::json;
pub use holochain_wasmer_common::string;
pub use holochain_wasmer_common::*;

#[macro_export]
macro_rules! memory_externs {
    () => {
        extern "C" {
            // memory stuff
            fn __import_allocation(
                guest_allocation_ptr: AllocationPtr,
                host_allocation_ptr: AllocationPtr,
            );
            fn __import_bytes(host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr);
        }
    };
}
memory_externs!();

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( fn $func_name(guest_allocation_ptr: $crate::AllocationPtr) -> $crate::AllocationPtr; )*
        }
    };
}

/// given a pointer to an allocation on the host, copy the allocation into the guest and return the
/// guest's pointer to it
pub fn map_bytes(host_allocation_ptr: Ptr) -> AllocationPtr {
    let tmp_allocation_ptr = allocation::allocate(allocation::ALLOCATION_BYTES_ITEMS as Len);
    unsafe {
        __import_allocation(tmp_allocation_ptr, host_allocation_ptr);
    };
    // this allocation has the correct length but host bytes ptr
    let [_, len] = allocation::from_allocation_ptr(tmp_allocation_ptr);
    allocation::deallocate(tmp_allocation_ptr, len);
    let guest_bytes_ptr = allocation::allocate(len);
    unsafe {
        __import_bytes(host_allocation_ptr, guest_bytes_ptr);
    };
    allocation::to_allocation_ptr([guest_bytes_ptr, len])
}

#[macro_export]
/// given a host allocation pointer and a type that implements TryFrom<JsonString>
/// - map bytes from the host into the guest
/// - restore a JsonString from the mapped bytes
/// - try to deserialize the given type from the restored JsonString
/// - if the deserialization fails, short circuit (return early) with a WasmError
/// - if everything is Ok, return the restored data as a native rust type inside the guest
macro_rules! host_args {
    ( $ptr:ident, $type:ty ) => {{
        use std::convert::TryInto;

        let val: $type =
            match $crate::json::from_allocation_ptr(holochain_wasmer_guest::map_bytes($ptr))
                .try_into()
            {
                Ok(v) => v,
                Err(_) => {
                    $crate::allocation::deallocate_from_allocation_ptr($ptr);
                    return $crate::json::to_allocation_ptr(
                        $crate::result::WasmResult::Err(
                            $crate::result::WasmError::ArgumentDeserializationFailed,
                        )
                        .into(),
                    );
                }
            };
        val
    }};
}

#[macro_export]
macro_rules! host_string {
    ( $ptr:ident ) => {{
        $crate::string::from_allocation_ptr($crate::map_bytes($ptr))
    }};
}

#[macro_export]
macro_rules! host_call {
    ($func_name:ident, $input:ident) => {{
        let host_allocation_ptr = unsafe {
            $func_name($crate::allocation::to_allocation_ptr([
                $input.as_ptr() as $crate::Ptr,
                $input.len() as $crate::Len,
            ]))
        };
        host_string!(host_allocation_ptr)
    }};
}

#[macro_export]
macro_rules! ret {
    ( $e: expr) => {{
        // enforce that everything be a ribosome result
        let r: $crate::result::WasmResult = $e;
        return holochain_wasmer_guest::json::to_allocation_ptr(r.into());
    }};
}
