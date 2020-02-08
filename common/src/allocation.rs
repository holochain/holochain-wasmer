use crate::AllocationPtr;
use std::mem;
use std::slice;

/// Allocation is a 2 item u64 slice of offset/length
pub const ALLOCATION_ITEMS: usize = 2;
pub type Allocation = [u64; ALLOCATION_ITEMS];

/// Need Allocation to be u8 to copy as bytes across host/guest
pub const ALLOCATION_BYTES_ITEMS: usize = 16;
pub type AllocationBytes = [u8; ALLOCATION_BYTES_ITEMS];

pub fn to_allocation_ptr(allocation: Allocation) -> AllocationPtr {
    // the allocation must exist as a vector or it will be dropped
    // slices drop even with ManuallyDrop
    let allocation_vec = vec![allocation[0], allocation[1]];
    let allocation_ptr = allocation_vec.as_ptr() as AllocationPtr;
    mem::ManuallyDrop::new(allocation_vec);
    allocation_ptr
}

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> Allocation {
    let slice = unsafe { slice::from_raw_parts(allocation_ptr as _, ALLOCATION_ITEMS) };
    [slice[0], slice[1]]
}

#[cfg(test)]
pub mod tests {

    use crate::allocation;
    use crate::Len;
    use crate::Ptr;

    #[test]
    fn allocate_allocation_ptr_test() {
        let some_ptr = 50 as Ptr;
        let some_len = 100 as Len;

        let allocation_ptr = allocation::to_allocation_ptr([some_ptr, some_len]);

        let restored_allocation = allocation::from_allocation_ptr(allocation_ptr);

        assert_eq!([some_ptr, some_len], restored_allocation,);
    }
}
