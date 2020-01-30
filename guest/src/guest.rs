
extern crate wee_alloc;
use std::mem;
use std::slice;
use common::bits_n_pieces::u64_merge_bits;
use common::memory::StringPtr;
use common::memory::StringCap;
use common::memory::PtrLen;
use common::bits_n_pieces::u64_split_bits;

extern "C" {
    fn host_process_string(ptr: StringPtr, cap: StringCap) -> PtrLen;
}

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn mem_string(ptr: StringPtr, len: StringCap) -> String {
    unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(ptr as _, len as _)) }.into()
}

#[no_mangle]
/// hack to allow the host to allocate a string capacity and receive the pointer to write into it
/// from outside
pub extern "C" fn pre_alloc_string(cap: StringCap) -> StringPtr {
    // https://doc.rust-lang.org/std/string/struct.String.html#examples-8
    // Prevent automatically dropping the String's data
    mem::ManuallyDrop::new(Vec::with_capacity(cap as _)).as_ptr()
}

#[no_mangle]
pub extern "C" fn process_string(ptr: StringPtr, cap: StringCap) -> PtrLen {
    // get the string the host is trying to pass us out of memory
    // the ptr and cap line up with what was previously allocated with pre_alloc_string
    let s = mem_string(ptr, cap);


    // imported host function calls are always unsafe
    let (processed_ptr, processed_len) = u64_split_bits(unsafe { host_process_string(s.as_ptr(), s.len()) });
    let host_processed_string = mem_string(processed_ptr as _, processed_len as _);
    let guest_processed = format!("guest: {}", host_processed_string);
    u64_merge_bits(guest_processed.as_ptr() as _, guest_processed.len() as _)
}
