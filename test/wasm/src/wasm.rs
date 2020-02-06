extern crate wee_alloc;
use common::memory::Ptr;
use guest::host_call;
use common::memory::Len;
use common::memory::AllocationPtr;
use common::allocate::string_allocation_ptr;
use guest::host_string_from_host_allocation_ptr;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C" {
    // memory stuff
    fn __copy_allocation_to_guest(guest_allocation_ptr: AllocationPtr, host_allocation_ptr: AllocationPtr);
    fn __host_copy_string(host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr);

    // api
    fn __test_process_string(ptr: Ptr, cap: Len) -> AllocationPtr;
}

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string_from_host_allocation_ptr(host_allocation_ptr);

    let s = format!("guest: {}", s);
    let s = host_call!(__test_process_string, s);
    string_allocation_ptr(s)
}
