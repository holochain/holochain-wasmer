pub mod allocate;

extern crate wee_alloc;
use crate::allocate::allocate;
use common::allocate::allocate_allocation_ptr;
use common::allocate::allocation_from_allocation_ptr;
use common::allocate::string_from_allocation;
use common::memory::AllocationPtr;
use common::memory::Len;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;

extern "C" {
    // memory stuff
    fn __copy_allocation_to_guest(
        guest_allocation_ptr: AllocationPtr,
        host_allocation_ptr: AllocationPtr,
    );
    fn __host_copy_string(host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr);
}

/// given a pointer to an allocation on the host, copy the allocation into the guest and return the
/// guest's pointer to it
fn map_string(host_allocation_ptr: Ptr) -> AllocationPtr {
    let tmp_allocation_ptr = allocate(ALLOCATION_BYTES_ITEMS as Len);
    unsafe {
        __copy_allocation_to_guest(tmp_allocation_ptr, host_allocation_ptr);
    };
    // this allocation has the correct length but host string ptr
    let [_, len] = allocation_from_allocation_ptr(tmp_allocation_ptr);
    let guest_string_ptr = allocate(len);
    unsafe {
        __host_copy_string(host_allocation_ptr, guest_string_ptr);
    };
    allocate_allocation_ptr(guest_string_ptr, len)
}

pub fn host_string_from_host_allocation_ptr(host_allocation_ptr: Ptr) -> String {
    string_from_allocation(allocation_from_allocation_ptr(map_string(
        host_allocation_ptr,
    )))
}

#[macro_export]
macro_rules! host_call {
    ($func_name:ident, $input:ident) => {
        host_string_from_host_allocation_ptr(unsafe {
            $func_name($input.as_ptr() as Ptr, $input.len() as Len)
        })
    };
}
