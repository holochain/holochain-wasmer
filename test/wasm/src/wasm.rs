extern crate wee_alloc;
use holochain_wasmer_guest::host_call;
use holochain_wasmer_guest::bytes;
use holochain_wasmer_guest::*;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C" {
    // memory stuff
    fn __import_allocation(guest_allocation_ptr: AllocationPtr, host_allocation_ptr: AllocationPtr);
    fn __import_bytes(host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr);

    // api
    fn __test_process_string(guest_allocation_ptr: AllocationPtr) -> AllocationPtr;
}

pub fn host_string_from_host_allocation_ptr(host_allocation_ptr: Ptr) -> Result<String, Error> {
    Ok(String::from(std::str::from_utf8(
        &bytes::from_allocation_ptr(holochain_wasmer_guest::map_bytes(host_allocation_ptr)),
    )?))
}

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string_from_host_allocation_ptr(host_allocation_ptr).expect("failed to get host string");

    let s = format!("guest: {}", s);
    let s = host_call!(__test_process_string, s).expect("host call");
    bytes::to_allocation_ptr(s.into_bytes())
}
