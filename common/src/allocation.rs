use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use std::mem;
use std::slice;

/// Allocation is a 2 item u64 slice of offset/length
pub const ALLOCATION_ITEMS: usize = 2;
pub type Allocation = [u64; ALLOCATION_ITEMS];

/// Need Allocation to be u8 to copy as bytes across host/guest
pub const ALLOCATION_BYTES_ITEMS: usize = 16;
pub type AllocationBytes = [u8; ALLOCATION_BYTES_ITEMS];

#[no_mangle]
/// allocate a length of bytes that won't be dropped by the allocator
/// return the pointer to it so bytes can be written to the allocation
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
