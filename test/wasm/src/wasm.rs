extern crate wee_alloc;
// use common::Ptr;
// use guest::host_call;
// use common::Len;
use common::AllocationPtr;
use common::bytes;
// use guest::host_string_from_host_allocation_ptr;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C" {
    // memory stuff
    // fn __import_allocation(guest_allocation_ptr: AllocationPtr, host_allocation_ptr: AllocationPtr);
    // fn __import_bytes(host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr);

    // api
    // fn __test_process_string(ptr: Ptr, cap: Len) -> AllocationPtr;
}

#[no_mangle]
pub extern "C" fn process_string(_host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    // let s = host_string_from_host_allocation_ptr(host_allocation_ptr)?;

    // let s = format!("guest: {}", s);
    // let s = host_call!(__test_process_string, s)?;
    let s = String::from("foo");
    bytes::to_allocation_ptr(s.into_bytes())
}
