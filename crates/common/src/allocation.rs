use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use holochain_serialized_bytes::prelude::*;
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
    let dummy: Vec<u8> = Vec::with_capacity(len as _);
    let ptr = dummy.as_ptr() as Ptr;
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
        // this is the inverse of From<Allocation>
        // it deallocates what From<Allocation> did an unsafe allocation for
        let allocation_vec: Vec<u64> = unsafe {
            Vec::from_raw_parts(allocation_ptr.0 as _, ALLOCATION_ITEMS, ALLOCATION_ITEMS)
        };
        // this is a new allocation that will be handled by the rust allocator
        [allocation_vec[0], allocation_vec[1]]
    }
}

impl From<AllocationPtr> for SerializedBytes {
    fn from(allocation_ptr: AllocationPtr) -> SerializedBytes {
        let allocation = Allocation::from(allocation_ptr);
        let b: Vec<u8> = unsafe {
            Vec::from_raw_parts(allocation[0] as _, allocation[1] as _, allocation[1] as _)
        };
        SerializedBytes::from(UnsafeBytes::from(b))
    }
}

impl From<SerializedBytes> for AllocationPtr {
    fn from(sb: SerializedBytes) -> AllocationPtr {
        let bytes: Vec<u8> = UnsafeBytes::from(sb).into();
        let bytes_ptr = bytes.as_ptr() as Ptr;
        let bytes_len = bytes.len() as Len;
        std::mem::ManuallyDrop::new(bytes);
        let allocation: Allocation = [bytes_ptr, bytes_len];
        AllocationPtr::from(allocation)
    }
}

#[cfg(test)]
pub mod tests {

    use crate::allocation;
    use crate::allocation::Allocation;
    use crate::*;
    use holochain_serialized_bytes::prelude::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Foo(String);

    holochain_serial!(Foo);

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

        let ptr = allocation::allocate(len);

        let _vec_that_might_overwrite_the_allocation: Vec<u8> = vec![1, 2, 3];

        // this shows that the 3 bytes we allocated are all 0 as expected
        // this probably means that the allocation worked
        // @TODO actually this doesn't mean anything
        // https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation
        // > Vec will not specifically overwrite any data that is removed from it, but also won't
        // > specifically preserve it. Its uninitialized memory is scratch space that it may use
        // > however it wants. It will generally just do whatever is most efficient or otherwise
        // > easy to implement.
        // > Even if you zero a Vec's memory first, that may not actually happen because the
        // > optimizer does not consider this a side-effect that must be preserved.
        // let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };
        // assert_eq!(vec![0, 0, 0], slice);

        allocation::deallocate(ptr, len);

        let some_vec: Vec<u8> = vec![1_u8, 10_u8, 100_u8];

        // the new vec should have the same pointer as the original allocation after we deallocate
        assert_eq!(ptr, some_vec.as_ptr() as Ptr);

        let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };

        // the same sized slice at the same pointer now looks like some_vec
        assert_eq!(slice.to_vec(), some_vec);
    }

    #[test]
    fn allocation_ptr_round_trip() {
        let allocation: Allocation = [1, 2];
        let allocation_ptr: AllocationPtr = allocation.into();
        let remote_ptr: RemotePtr = allocation_ptr.as_remote_ptr();

        // can peek without any deallocations
        assert_eq!(allocation_ptr.peek_allocation(), [1, 2],);
        assert_eq!(allocation_ptr.peek_allocation(), [1, 2],);

        // can round trip back
        let returned_allocation: Allocation = allocation_ptr.into();

        assert_eq!(returned_allocation, [1, 2]);

        // round tripping above deallocates the original allocation
        // put something here to try and make sure memory doesn't stick around
        let _: Allocation = [3, 4];
        assert_ne!(
            AllocationPtr::from_remote_ptr(remote_ptr).peek_allocation(),
            [1, 2]
        );
    }

    #[test]
    fn serialized_bytes_from_allocation_test() {
        let foo: Foo = Foo("foo".into());
        let foo_clone = foo.clone();
        let foo_sb: SerializedBytes = foo.try_into().unwrap();
        let foo_sb_clone = foo_sb.clone();

        let ptr: AllocationPtr = foo_sb.into();
        let remote_ptr: RemotePtr = ptr.as_remote_ptr();

        // the Allocation should get deallocated so this should not match
        // after the
        let unexpected_allocation: Allocation = ptr.peek_allocation();

        // ownership of these bytes should be taken by SerializedBytes
        let inner_bytes: Vec<u8> = unsafe {
            std::slice::from_raw_parts(unexpected_allocation[0] as _, unexpected_allocation[1] as _)
        }
        .to_vec();

        let recovered_foo_sb: SerializedBytes = ptr.into();

        // the AllocationPtr's Allocation should be deallocated here
        assert_ne!(
            AllocationPtr::from_remote_ptr(remote_ptr).peek_allocation(),
            unexpected_allocation
        );

        assert_eq!(foo_sb_clone, recovered_foo_sb);

        let recovered_foo: Foo = recovered_foo_sb.try_into().unwrap();

        let inner_bytes_2: Vec<u8> = unsafe {
            std::slice::from_raw_parts(unexpected_allocation[0] as _, unexpected_allocation[1] as _)
        }
        .to_vec();

        // inner_bytes_2 should be nothing because inner_bytes was owned by SerializedBytes which
        // turned into a Foo
        assert_ne!(inner_bytes, inner_bytes_2,);

        assert_eq!(foo_clone, recovered_foo);
    }
}
