use holochain_wasmer_guest::*;

holochain_wasmer_guest::holochain_externs!();

// macro_rules! typed_externs {
//     ( $newtype:tt;

#[no_mangle]
pub extern "C" fn string_input_ignored_empty_ret(_: RemotePtr) -> RemotePtr {
    ret!(test_common::StringType::from(String::new()));
}

#[no_mangle]
pub extern "C" fn string_input_args_empty_ret(ptr: RemotePtr) -> RemotePtr {
    let _: test_common::StringType = host_args!(ptr);
    ret!(test_common::StringType::from(String::new()));
}

#[no_mangle]
pub extern "C" fn string_input_args_echo_ret(ptr: RemotePtr) -> RemotePtr {
    let r: test_common::StringType = host_args!(ptr);
    ret!(r);
}

#[no_mangle]
pub extern "C" fn string_serialize_large(_: RemotePtr) -> RemotePtr {
    let s = test_common::StringType::from(String::from(".".repeat(1_000_000)));
    let _: SerializedBytes = s.try_into().unwrap();
    ret!(test_common::StringType::from(String::new()));
}

#[no_mangle]
pub extern "C" fn string_ret_large(_: RemotePtr) -> RemotePtr {
    let s = test_common::StringType::from(String::from(".".repeat(1_000_000)));
    ret!(s);
}
