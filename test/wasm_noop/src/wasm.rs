use holochain_wasmer_guest::*;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

holochain_wasmer_guest::holochain_externs!();

#[no_mangle]
pub extern "C" fn a(_: RemotePtr) -> RemotePtr {
    ret!(test_common::StringType::from(String::new()));
}

#[no_mangle]
pub extern "C" fn b(ptr: RemotePtr) -> RemotePtr {
    let _: test_common::StringType = host_args!(ptr);
    ret!(test_common::StringType::from(String::new()));
}

#[no_mangle]
pub extern "C" fn c(ptr: RemotePtr) -> RemotePtr {
    let r: test_common::StringType = host_args!(ptr);
    ret!(r);
}
