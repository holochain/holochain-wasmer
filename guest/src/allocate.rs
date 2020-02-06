use common::memory::Len;
use common::memory::Ptr;
use std::mem;
use std::slice;

#[no_mangle]
/// allocate a length of bytes that won't be dropped by the allocator
/// return the pointer to it so the host can write to the allocation
pub extern "C" fn allocate(len: Len) -> Ptr {
    // https://doc.rust-lang.org/std/string/struct.String.html#examples-8
    // Prevent automatically dropping the String's data
    let dummy: Vec<u8> = Vec::with_capacity(len as _);
    let ptr = dummy.as_slice().as_ptr() as Ptr;
    mem::ManuallyDrop::new(dummy);
    ptr
}

/// restore an allocation so that it is dropped immediately
/// this needs to be called on anything allocated above as the allocator
/// will never free the memory otherwise
pub extern "C" fn deallocate(ptr: Ptr, len: Len) {
    let _: &[u8] = unsafe { slice::from_raw_parts(ptr as _, len as _) };
}
