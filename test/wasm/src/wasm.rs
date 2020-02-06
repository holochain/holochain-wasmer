extern crate wee_alloc;
use common::memory::Ptr;
use guest::allocate::allocate;
use guest::host_call;
use common::memory::ALLOCATION_BYTES_ITEMS;
use common::memory::Len;
use common::memory::AllocationPtr;
use common::allocate::string_allocation_ptr;
use common::allocate::allocation_from_allocation_ptr;
use common::allocate::string_from_allocation;
use common::allocate::allocate_allocation_ptr;

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

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string_from_host_allocation_ptr(host_allocation_ptr);

    // imported host function calls are always unsafe
    let s = format!("guest: {}", s);
    let s = host_call!(__test_process_string, s);
    string_allocation_ptr(s)
}
