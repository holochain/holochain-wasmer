pub mod allocate;

extern crate wee_alloc;
use crate::allocate::allocate;
use common::allocation;
use common::bytes;
use common::error::Error;
use common::AllocationPtr;
use common::Len;
use common::Ptr;

extern "C" {
    // memory stuff
    fn __import_allocation(guest_allocation_ptr: AllocationPtr, host_allocation_ptr: AllocationPtr);
    fn __import_bytes(host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr);
}

/// given a pointer to an allocation on the host, copy the allocation into the guest and return the
/// guest's pointer to it
fn map_string(host_allocation_ptr: Ptr) -> AllocationPtr {
    let tmp_allocation_ptr = allocate(allocation::ALLOCATION_BYTES_ITEMS as Len);
    unsafe {
        __import_allocation(tmp_allocation_ptr, host_allocation_ptr);
    };
    // this allocation has the correct length but host string ptr
    let [_, len] = allocation::from_allocation_ptr(tmp_allocation_ptr);
    let guest_bytes_ptr = allocate(len);
    unsafe {
        __import_bytes(host_allocation_ptr, guest_bytes_ptr);
    };
    allocation::to_allocation_ptr([guest_bytes_ptr, len])
}

pub fn host_string_from_host_allocation_ptr(host_allocation_ptr: Ptr) -> Result<String, Error> {
    Ok(String::from(std::str::from_utf8(
        &bytes::from_allocation_ptr(map_string(host_allocation_ptr)),
    )?))
}

#[macro_export]
macro_rules! host_call {
    ($func_name:ident, $input:ident) => {
        host_string_from_host_allocation_ptr(unsafe {
            $func_name($input.as_ptr() as Ptr, $input.len() as Len)
        })
    };
}
