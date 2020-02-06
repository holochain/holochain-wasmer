pub mod allocate;

extern crate wee_alloc;
use common::memory::Ptr;
use crate::allocate::allocate;
use common::memory::ALLOCATION_BYTES_ITEMS;
use common::memory::Len;
use common::memory::AllocationPtr;
use common::allocate::string_allocation_ptr;
use common::allocate::allocation_from_allocation_ptr;
use common::allocate::string_from_allocation;
use common::allocate::allocate_allocation_ptr;

extern "C" {
    // memory stuff
    fn __copy_allocation_to_guest(guest_allocation_ptr: AllocationPtr, host_allocation_ptr: AllocationPtr);
    fn __host_copy_string(host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr);
    fn __prn(u: u64);

    // api
    fn __host_process_string(ptr: Ptr, cap: Len) -> AllocationPtr;
}

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// given a pointer to an allocation on the host, copy the allocation into the guest and return the
/// guest's pointer to it
fn map_string(host_allocation_ptr: Ptr) -> AllocationPtr {
    let tmp_allocation_ptr = allocate(ALLOCATION_BYTES_ITEMS as Len);
    unsafe { __copy_allocation_to_guest(tmp_allocation_ptr, host_allocation_ptr); };
    // this allocation has the correct length but host string ptr
    let [_, len] = allocation_from_allocation_ptr(tmp_allocation_ptr);
    let guest_string_ptr = allocate(len);
    unsafe { __host_copy_string(host_allocation_ptr, guest_string_ptr); };
    allocate_allocation_ptr(guest_string_ptr, len)
}

pub fn host_string_from_host_allocation_ptr(host_allocation_ptr: Ptr) -> String {
    string_from_allocation(allocation_from_allocation_ptr(map_string(host_allocation_ptr)))
}

macro_rules! host_call {
    ($func_name:ident, $input:ident) => {
        host_string_from_host_allocation_ptr(
            unsafe {
                $func_name(
                    $input.as_ptr() as Ptr,
                    $input.len() as Len,
                )
            }
        )
    };
}

pub fn host_process_string(s: String) -> String {
    host_call!(__host_process_string, s)
}

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string_from_host_allocation_ptr(host_allocation_ptr);

    // imported host function calls are always unsafe
    let s = format!("guest: {}", s);
    let s = host_process_string(s);
    string_allocation_ptr(s)
}
