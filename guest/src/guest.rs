pub mod allocate;

extern crate wee_alloc;
use std::slice;
use std::convert::TryInto;
use common::memory::Ptr;
use crate::allocate::allocate;
use common::memory::ALLOCATION_ITEMS;
use common::memory::Len;
use common::memory::AllocationPtr;
use common::allocate::string_allocation_ptr;
use common::allocate::allocation_from_allocation_ptr;

extern "C" {
    fn __host_process_string(ptr: Ptr, cap: Len) -> Ptr;
    fn __copy_allocation_to_guest(guest_ptr: Ptr, allocation_ptr: Ptr);
    fn __host_copy_string(host_ptr: Ptr, guest_ptr: Ptr, len: Len);
}

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn host_string(ptr: Ptr, len: Len) -> String {
    let slice = unsafe { slice::from_raw_parts(ptr as _, len as _) };
    String::from(std::str::from_utf8(slice).unwrap())
}

pub fn host_string_from_allocation_ptr(allocation_ptr: Ptr) -> String {
    let guest_ptr = allocate(ALLOCATION_ITEMS as Len);
    let _ = unsafe { __copy_allocation_to_guest(guest_ptr, allocation_ptr); };
    let allocation = allocation_from_allocation_ptr(guest_ptr);

    let guest_string_ptr = allocate(allocation[1].try_into().unwrap());
    unsafe { __host_copy_string(allocation[0] as _, guest_string_ptr as _, allocation[1] as _) };
    host_string(guest_string_ptr as _, allocation[1] as _)
}

macro_rules! host_call {
    ($func_name:ident, $input:ident) => {
        host_string_from_allocation_ptr(
            unsafe {
                $func_name(
                    $input.as_ptr() as u64,
                    $input.len() as u64
                )
            }
        )
    };
}

pub fn host_process_string(s: String) -> String {
    // let host_processed_position_ptr = unsafe { __host_process_string(s.as_ptr() as _, s.len() as _) };
    //
    // host_string_from_allocation_ptr
    host_call!(__host_process_string, s)
}

#[no_mangle]
pub extern "C" fn process_string(ptr: Ptr, len: Len) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string(ptr, len);

    // imported host function calls are always unsafe
    let processed = format!("guest: {}", s);
    // let host_processed = host_process_string(guest_processed);
    string_allocation_ptr(processed)
}
