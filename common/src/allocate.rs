use std::mem;
use crate::memory::Ptr;
use crate::memory::AllocationPtr;
use crate::memory::Len;

pub fn allocate_allocation_ptr(ptr: Ptr, len: Len) -> AllocationPtr {
    // the allocation must start life as a vector or it will be dropped
    // slices drop even with ManuallyDrop
    let allocation_vec = vec![ptr, len];
    let allocation_ptr = allocation_vec.as_ptr() as AllocationPtr;
    mem::ManuallyDrop::new(allocation_vec);
    allocation_ptr
}

pub fn string_allocation_ptr(s: String) -> AllocationPtr {
    let s_ptr = s.as_ptr() as Ptr;
    let s_len = s.len() as Len;
    mem::ManuallyDrop::new(s);

    allocate_allocation_ptr(s_ptr, s_len)
}
