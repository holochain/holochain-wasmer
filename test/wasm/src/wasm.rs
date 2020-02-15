extern crate wee_alloc;
extern crate test_common;

use holochain_wasmer_guest::bytes;
use holochain_wasmer_guest::*;
use test_common::SomeStruct;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// define the host functions we require in order to pull/push data across the host/guest boundary
memory_externs!();

// define a few functions we expect the host to provide for us
host_externs!(__this_func_doesnt_exist_but_we_can_extern_it_anyway, __test_process_string);

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string!(host_allocation_ptr);

    let s = format!("guest: {}", s);
    let s = host_call!(__test_process_string, s);
    bytes::to_allocation_ptr(s.into_bytes())
}

#[no_mangle]
pub extern "C" fn stacked_strings(_: AllocationPtr) -> AllocationPtr {
    // get the first string allocated to be returned
    let first = "first";
    // the second string allocation should do nothing to the first
    let _second = "second";

    string::to_allocation_ptr(first.into())
}

#[no_mangle]
pub extern "C" fn some_ret(_: AllocationPtr) -> AllocationPtr {
    ret!(SomeStruct::new("foo".into()));
}

#[no_mangle]
pub extern "C" fn some_ret_err(_: AllocationPtr) -> AllocationPtr {
    ret_err!("oh no!");
}

#[no_mangle]
pub extern "C" fn native_type(host_allocation_ptr: AllocationPtr) -> AllocationPtr {
    let input = host_args!(host_allocation_ptr, SomeStruct);
    ret!(input);
}

#[no_mangle]
pub extern "C" fn try_result_succeeds(_: AllocationPtr) -> AllocationPtr {
    let ok: Result<SomeStruct, ()> = Ok(SomeStruct::new("foo".into()));
    let result = try_result!(ok, "this can't fail");
    ret!(result);
}

#[no_mangle]
pub extern "C" fn try_result_fails_fast(_: AllocationPtr) -> AllocationPtr {
    try_result!(Err(()), "it fails!");
    string::to_allocation_ptr("this never happens".into())
}
