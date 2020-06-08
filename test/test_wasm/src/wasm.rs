extern crate test_common;

use holochain_wasmer_guest::*;
use test_common::BytesType;
use test_common::SomeStruct;
use test_common::StringType;

// define the host functions we require in order to pull/push data across the host/guest boundary
memory_externs!();

// define a few functions we expect the host to provide for us
host_externs!(
    __debug,
    __noop,
    __this_func_doesnt_exist_but_we_can_extern_it_anyway,
    __test_process_string,
    __test_process_struct
);

pub fn result_support() -> Result<(), WasmError> {
    // want to show here that host_call!() supports the ? operator
    // this is needed if we are to call host functions outside the externed functions that can only
    // return AllocationPtrs
    let _: SomeStruct = host_call!(__noop, ())?;

    Ok(())
}

#[no_mangle]
pub extern "C" fn process_bytes(host_allocation_ptr: RemotePtr) -> RemotePtr {
    let b: BytesType = host_args!(host_allocation_ptr);
    let mut b = b.inner();
    let mut more_bytes = vec![50_u8, 60_u8, 70_u8, 80_u8];
    b.append(&mut more_bytes);
    let b = BytesType::from(b);
    ret!(b);
}

#[no_mangle]
pub extern "C" fn process_string(host_allocation_ptr: RemotePtr) -> RemotePtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s: StringType = host_args!(host_allocation_ptr);

    let s = StringType::from(format!("guest: {}", String::from(s)));
    let s: StringType = try_result!(
        host_call!(__test_process_string, s),
        "could not __test_process_string"
    );
    ret!(s);
}

#[no_mangle]
pub extern "C" fn process_native(host_allocation_ptr: RemotePtr) -> RemotePtr {
    let input: SomeStruct = host_args!(host_allocation_ptr);
    let processed: SomeStruct = try_result!(
        host_call!(__test_process_struct, input),
        "could not deserialize SomeStruct in process_native"
    );
    ret!(processed);
}

#[no_mangle]
pub extern "C" fn stacked_strings(_: RemotePtr) -> RemotePtr {
    // get the first string allocated to be returned
    let first = "first";
    // the second string allocation should do nothing to the first
    let _second = "second";

    ret!(StringType::from(String::from(first)));
}

#[no_mangle]
pub extern "C" fn some_ret(_: RemotePtr) -> RemotePtr {
    ret!(SomeStruct::new("foo".into()));
}

#[no_mangle]
pub extern "C" fn some_ret_err(_: RemotePtr) -> RemotePtr {
    ret_err!("oh no!");
}

#[no_mangle]
pub extern "C" fn native_type(host_allocation_ptr: RemotePtr) -> RemotePtr {
    let input: SomeStruct = host_args!(host_allocation_ptr);
    ret!(input);
}

#[no_mangle]
pub extern "C" fn try_result_succeeds(_: RemotePtr) -> RemotePtr {
    let ok: Result<SomeStruct, ()> = Ok(SomeStruct::new("foo".into()));
    let result = try_result!(ok, "this can't fail");
    ret!(result);
}

#[no_mangle]
pub extern "C" fn try_result_fails_fast(_: RemotePtr) -> RemotePtr {
    try_result!(Err(()), "it fails!");
    ret!(());
}
