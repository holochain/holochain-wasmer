use std::mem;
use crate::memory::Ptr;
use crate::memory::AllocationPtr;
use crate::memory::Len;
use crate::memory::Allocation;
use std::slice;
use crate::memory::ALLOCATION_ITEMS;

pub fn allocate_allocation_ptr(ptr: Ptr, len: Len) -> AllocationPtr {
    // the allocation must start life as a vector or it will be dropped
    // slices drop even with ManuallyDrop
    let allocation_vec = vec![ptr, len];
    let allocation_ptr = allocation_vec.as_ptr() as AllocationPtr;
    mem::ManuallyDrop::new(allocation_vec);
    allocation_ptr
}

pub fn allocation_from_allocation_ptr(allocation_ptr: AllocationPtr) -> Allocation {
    let slice = unsafe { slice::from_raw_parts(allocation_ptr as _, ALLOCATION_ITEMS) };
    [slice[0], slice[1]]
}

pub fn string_allocation_ptr(s: String) -> AllocationPtr {
    let s_ptr = s.as_ptr() as Ptr;
    let s_len = s.len() as Len;
    mem::ManuallyDrop::new(s);

    allocate_allocation_ptr(s_ptr, s_len)
}

#[cfg(test)]
pub mod tests {

    use crate::allocate::allocation_from_allocation_ptr;
    use crate::allocate::allocate_allocation_ptr;
    use crate::memory::Len;
    use crate::memory::Ptr;

    #[test]
    fn allocate_allocation_ptr_test() {
        let some_ptr = 50 as Ptr;
        let some_len = 100 as Len;

        let allocation_ptr = allocate_allocation_ptr(some_ptr, some_len);

        let restored_allocation = allocation_from_allocation_ptr(allocation_ptr);

        assert_eq!(
            [some_ptr, some_len],
            restored_allocation,
        );
    }

}
