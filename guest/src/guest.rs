
extern crate wee_alloc;
use std::mem;
use std::slice;
use common::bits_n_pieces::u64_merge_bits;
// use common::memory::StringPtr;
// use common::memory::StringCap;
// use common::memory::PtrLen;
// use common::bits_n_pieces::u64_split_bits;

extern "C" {
    // fn host_process_string(ptr: StringPtr, cap: StringCap) -> PtrLen;
}

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn host_string(ptr: u32, len: u32) -> String {
    let slice = unsafe { slice::from_raw_parts(ptr as _, len as _) };
    String::from(std::str::from_utf8(slice).unwrap())
    // String::from_raw_parts(ptr, cap as _, cap as _)
}

#[no_mangle]
/// hack to allow the host to allocate a string capacity and receive the pointer to write into it
/// from outside
pub extern "C" fn pre_alloc_string(cap: u32) -> i32 {
    // https://doc.rust-lang.org/std/string/struct.String.html#examples-8
    // Prevent automatically dropping the String's data
    let dummy:Vec<u8> = Vec::with_capacity(cap as _);
    let ptr = dummy.as_slice().as_ptr() as i32;
    mem::ManuallyDrop::new(dummy);
    // mem::forget(dummy);
    ptr
}

pub fn prepare_return(s: String) -> u64 {
    let s_ptr = s.as_ptr();
    let s_len = s.len();
    mem::ManuallyDrop::new(s);
    u64_merge_bits(s_ptr as _, s_len as _)
}

#[no_mangle]
pub extern "C" fn process_string(ptr: u32, cap: u32) -> u64 {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = host_string(ptr, cap);

    // imported host function calls are always unsafe
    // let (processed_ptr, processed_len) = u64_split_bits(unsafe { host_process_string(s.as_ptr(), s.len()) });
    // let host_processed_string = host_string(processed_ptr as _, processed_len as _);
    let guest_processed = format!("guest: {}", s);
    prepare_return(guest_processed)
}
