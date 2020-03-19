use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use std::mem;

/// Allocation is a 2 item u64 slice of offset/length
pub const ALLOCATION_ITEMS: usize = 2;
pub type Allocation = [u64; ALLOCATION_ITEMS];

/// Need Allocation to be u8 to copy as bytes across host/guest
/// the u64 integers of an allocation are broken down into u8 bytes to copy into wasm memory
pub const ALLOCATION_BYTES_ITEMS: usize = 16;
pub type AllocationBytes = [u8; ALLOCATION_BYTES_ITEMS];

#[no_mangle]
/// allocate a length of bytes that won't be dropped by the allocator
/// return the pointer to it so bytes can be written to the allocation
pub extern "C" fn allocate(len: Len) -> Ptr {
    // https://doc.rust-lang.org/std/string/struct.String.html#examples-8
    let dummy: Vec<u8> = Vec::with_capacity(len as _);
    let ptr = dummy.as_slice().as_ptr() as Ptr;
    mem::ManuallyDrop::new(dummy);
    ptr
}

/// restore an allocation so that it is dropped immediately
/// this needs to be called on anything allocated above as the allocator
/// will never free the memory otherwise
pub extern "C" fn deallocate<'a>(ptr: Ptr, len: Len) {
    let _: Vec<u8> = unsafe { Vec::from_raw_parts(ptr as _, len as _, len as _) };
}

/// given an Allocation returns a u64 pointer to it
/// the Allocation (slice) is internally converted to a Vec<u8> that requires a manual drop
/// i.e. the allocation vec is not dropped when it goes out of scope
/// the pointer returned is to this new vector and the originally passed allocation is handled by
/// the rust allocater as per normal ownership rules
/// this allows us to pass the AllocationPtr across the host/guest boundary in either direction and
/// have the Allocation bytes remain in memory until the AllocationPtr recipient is ready to have
/// them copied in.
/// the from_allocation_ptr function does the inverse and internally deallocates the allocation
/// vector that is created here.
/// to avoid memory leaks, every Allocation must fully round-trip through these two functions.
impl From<Allocation> for AllocationPtr {
    fn from(allocation: Allocation) -> AllocationPtr {
        // the allocation must exist as a vector or it will be dropped
        // slices drop even with ManuallyDrop
        let allocation_vec: Vec<u64> = vec![allocation[0], allocation[1]];
        let allocation_ptr = allocation_vec.as_ptr() as Ptr;
        mem::ManuallyDrop::new(allocation_vec);
        AllocationPtr(allocation_ptr)
    }
}

impl From<AllocationPtr> for Allocation {
    fn from(allocation_ptr: AllocationPtr) -> Allocation {
        // this is the inverse of to_allocation_ptr
        // it deallocates what to_allocation_ptr did an unsafe allocation for
        let allocation_vec: Vec<u64> = unsafe {
            Vec::from_raw_parts(allocation_ptr.0 as _, ALLOCATION_ITEMS, ALLOCATION_ITEMS)
        };
        // this is a new allocation that will be handled by the rust allocator
        [allocation_vec[0], allocation_vec[1]]
    }
}

impl AllocationPtr {
    /// get the Allocation for this Allocation _without_ deallocating the Allocation in the process
    /// usually you do not want to do this because From<AllocationPtr> for Allocation consumes the
    /// original AllocationPtr and returns a new identical Allocation
    pub fn peek_allocation(&self) -> Allocation {
        let allocation_slice: &[u64] =
            unsafe { std::slice::from_raw_parts(self.0 as _, ALLOCATION_ITEMS) };
        [allocation_slice[0], allocation_slice[1]]
    }
}

#[cfg(test)]
pub mod tests {

    use crate::allocation;
    use crate::allocation::Allocation;
    use crate::AllocationPtr;
    use crate::Len;
    use crate::Ptr;

    #[test]
    fn allocate_allocation_ptr_test() {
        let some_ptr = 50 as Ptr;
        let some_len = 100 as Len;

        let allocation_ptr = AllocationPtr::from([some_ptr, some_len]);

        let restored_allocation: Allocation = allocation_ptr.into();

        assert_eq!([some_ptr, some_len], restored_allocation,);
    }

    #[test]
    fn dellocate_test() {
        let len = 3 as Len;

        let expected: Vec<u8> = vec![0, 0, 0];
        let ptr = allocation::allocate(len);

        println!("first alloc {} {}", ptr, len);

        let _vec_that_might_overwrite_the_allocation: Vec<u8> = vec![1, 2, 3];

        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };

        // this shows that the 3 bytes we allocated are all 0 as expected
        // this probably means that the allocation worked
        assert_eq!(expected, slice);

        allocation::deallocate(ptr, len);

        let some_vec: Vec<u8> = vec![1_u8, 10_u8, 100_u8];

        // the new vec should have the same pointer as the original allocation after we deallocate
        assert_eq!(ptr, some_vec.as_ptr() as Ptr);

        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };

        // the same sized slice at the same pointer now looks like some_vec
        assert_eq!(slice.to_vec(), some_vec);
    }
}
