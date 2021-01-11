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
    let _: SomeStruct = host_call(__noop, &())?;

    Ok(())
}

#[no_mangle]
pub extern "C" fn process_bytes(guest_ptr: GuestPtr) -> GuestPtr {
    let b: BytesType = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    let mut b = b.inner();
    let mut more_bytes = vec![50_u8, 60_u8, 70_u8, 80_u8];
    b.append(&mut more_bytes);
    let b = BytesType::from(b);
    return_ptr(b)
}

#[no_mangle]
pub extern "C" fn process_string(guest_ptr: GuestPtr) -> GuestPtr {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s: StringType = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };

    let s: String = format!("guest: {}", String::from(s));
    let s: StringType = try_ptr!(
        host_call::<&String, StringType>(__test_process_string, &s),
        "could not __test_process_string"
    );
    return_ptr(s)
}

#[no_mangle]
pub extern "C" fn process_native(guest_ptr: GuestPtr) -> GuestPtr {
    let input: SomeStruct = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    let processed: SomeStruct = try_ptr!(
        host_call(__test_process_struct, &input),
        "could not deserialize SomeStruct in process_native"
    );
    return_ptr(processed)
}

#[no_mangle]
pub extern "C" fn stacked_strings(guest_ptr: GuestPtr) -> GuestPtr {
    let _: () = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    // get the first string allocated to be returned
    let first = "first";
    // the second string allocation should do nothing to the first
    let _second = "second";

    return_ptr(String::from(first))
}

#[no_mangle]
pub extern "C" fn some_ret(guest_ptr: GuestPtr) -> GuestPtr {
    let _: () = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    return_ptr(SomeStruct::new("foo".into()))
}

#[no_mangle]
pub extern "C" fn some_ret_err(guest_ptr: GuestPtr) -> GuestPtr {
    let _: () = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    return_err_ptr("oh no!")
}

#[no_mangle]
pub extern "C" fn native_type(guest_ptr: GuestPtr) -> GuestPtr {
    let input: SomeStruct = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    return_ptr(input)
}

#[no_mangle]
pub extern "C" fn try_ptr_succeeds(guest_ptr: GuestPtr) -> GuestPtr {
    let _: () = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    let ok: Result<SomeStruct, ()> = Ok(SomeStruct::new("foo".into()));
    let result: Result<SomeStruct, ()> = Ok(try_ptr!(ok, "this can't fail"));
    return_ptr(result)
}

#[no_mangle]
pub extern "C" fn try_ptr_fails_fast(guest_ptr: GuestPtr) -> GuestPtr {
    let _: () = match host_args(guest_ptr) {
        Ok(v) => v,
        Err(err_ptr) => return err_ptr,
    };
    let result: Result<(), WasmError> = Ok(try_ptr!(Err(()), "it fails!"));
    return_ptr(result)
}
