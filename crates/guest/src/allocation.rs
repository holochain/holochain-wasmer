use holochain_wasmer_common::fat_ptr;
use holochain_wasmer_common::*;
use std::mem;

#[no_mangle]
/// allocate a length of bytes that won't be dropped by the allocator
/// return the pointer to it so bytes can be written to the allocation
pub extern "C" fn allocate(len: Len) -> GuestPtr {
    let dummy: Vec<u8> = Vec::with_capacity(len as _);
    let ptr = dummy.as_ptr() as GuestPtr;
    mem::ManuallyDrop::new(dummy);
    ptr
}

/// restore an allocation so that it is dropped immediately
/// this needs to be called on anything allocated above as the allocator
/// will never free the memory otherwise
pub extern "C" fn deallocate<'a>(ptr: GuestPtr, len: Len) {
    let _: Vec<u8> = unsafe { Vec::from_raw_parts(ptr as _, len as _, len as _) };
}

/// AllocationPtr wraps a ptr that is used to pass the location of an Allocation
/// between the host and guest (in either direction).
/// The AllocationPtr intentionally does not implement Clone
/// The From<Allocation> and Into<Allocation> round trip handles manually allocating
/// and deallocating an internal vector that is shared across host/guest
/// If the AllocationPtr was to be cloned the shared vector could be allocated and
/// deallocated in an undefined way
pub struct AllocationPtr(GuestPtr);

impl AllocationPtr {
    /// normally we don't want to expose the inner Ptr because cloning or reusing it
    /// can lead to bad allocation and deallocation
    /// in the case that we pass the inner Ptr over to the host we do need to access it
    /// this function is just like From<AllocationPtr> for Ptr but renamed to make it
    /// clear that it has only one valid use-case
    pub fn as_guest_ptr(&self) -> GuestPtr {
        self.0
    }

    pub fn from_guest_ptr(guest_ptr: GuestPtr) -> Self {
        Self(guest_ptr)
    }
}

/// given an WasmFatPtr returns an AllocationPtr pointer to it
/// the WasmFatPtr is internally converted to a Vec<u8> that requires a manual drop
///
/// this is analagous to Box::into_raw()
///
/// i.e. the allocation vec is not dropped when it goes out of scope
/// the pointer returned is to this new vector and the originally passed allocation is handled by
/// the rust allocater as per normal ownership rules
/// this allows us to pass the AllocationPtr across the host/guest boundary in either direction and
/// have the WasmFatPtr bytes remain in memory until the AllocationPtr recipient is ready to have
/// them copied in.
/// the From<AllocationPtr> function does the inverse and internally deallocates the allocation
/// vector that is created here.
/// to avoid memory leaks, every WasmFatPtr must fully round-trip through these two functions.
impl From<fat_ptr::WasmFatPtr> for AllocationPtr {
    fn from(fat_ptr: fat_ptr::WasmFatPtr) -> Self {
        // the allocation must exist as a vector or it will be dropped
        // slices drop even with ManuallyDrop
        let allocation_vec: Vec<GuestPtr> = vec![fat_ptr.ptr_offset(), fat_ptr.ptr_len()];
        let allocation_ptr = allocation_vec.as_ptr() as GuestPtr;
        mem::ManuallyDrop::new(allocation_vec);
        AllocationPtr(allocation_ptr)
    }
}

/// inverse of From<fat_ptr::WasmFatPtr> for AllocationPtr
///
/// unleaks the previously leaked memory
///
/// @see From<WasmFatPtr> for AllocationPtr
impl From<AllocationPtr> for fat_ptr::WasmFatPtr {
    fn from(allocation_ptr: AllocationPtr) -> Self {
        // this is the inverse of From<WasmFatPtr>
        // it deallocates what From<WasmFatPtr> did an unsafe allocation for
        let allocation_vec: Vec<GuestPtr> = unsafe {
            Vec::from_raw_parts(
                allocation_ptr.0 as _,
                fat_ptr::WASM_FAT_PTR_ITEMS,
                fat_ptr::WASM_FAT_PTR_ITEMS,
            )
        };
        // this is a new allocation that will be handled by the rust allocator
        [allocation_vec[0], allocation_vec[1]].into()
    }
}

impl From<AllocationPtr> for SerializedBytes {
    fn from(allocation_ptr: AllocationPtr) -> SerializedBytes {
        let fat_ptr = fat_ptr::WasmFatPtr::from(allocation_ptr);
        let b: Vec<u8> = unsafe {
            Vec::from_raw_parts(
                fat_ptr.ptr_offset() as _,
                fat_ptr.ptr_len() as _,
                fat_ptr.ptr_len() as _,
            )
        };
        SerializedBytes::from(UnsafeBytes::from(b))
    }
}

impl From<SerializedBytes> for AllocationPtr {
    fn from(sb: SerializedBytes) -> AllocationPtr {
        let bytes: Vec<u8> = UnsafeBytes::from(sb).into();
        let bytes_ptr = bytes.as_ptr() as GuestPtr;
        let bytes_len = bytes.len() as Len;
        std::mem::ManuallyDrop::new(bytes);
        // move through a fat pointer to leak correctly
        let fat_ptr: fat_ptr::WasmFatPtr = [bytes_ptr, bytes_len].into();
        AllocationPtr::from(fat_ptr)
    }
}

#[cfg(test)]
pub mod tests {

    // use crate::allocation;
    // use crate::allocation::Allocation;
    // use crate::*;
    use holochain_serialized_bytes::prelude::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, SerializedBytes)]
    struct Foo(String);

    // #[test]
    // fn allocate_allocation_ptr_test() {
    //     let some_ptr = 50 as Ptr;
    //     let some_len = 100 as Len;
    //
    //     let allocation_ptr = AllocationPtr::from([some_ptr, some_len]);
    //
    //     let restored_allocation: Allocation = allocation_ptr.into();
    //
    //     assert_eq!([some_ptr, some_len], restored_allocation,);
    // }

    // #[test]
    // fn dellocate_test() {
    //     let len = 3 as Len;
    //
    //     let ptr = allocation::allocate(len);
    //
    //     let _vec_that_might_overwrite_the_allocation: Vec<u8> = vec![1, 2, 3];
    //
    //     // this shows that the 3 bytes we allocated are all 0 as expected
    //     // this probably means that the allocation worked
    //     // @TODO actually this doesn't mean anything
    //     // https://doc.rust-lang.org/std/vec/struct.Vec.html#capacity-and-reallocation
    //     // > Vec will not specifically overwrite any data that is removed from it, but also won't
    //     // > specifically preserve it. Its uninitialized memory is scratch space that it may use
    //     // > however it wants. It will generally just do whatever is most efficient or otherwise
    //     // > easy to implement.
    //     // > Even if you zero a Vec's memory first, that may not actually happen because the
    //     // > optimizer does not consider this a side-effect that must be preserved.
    //     // let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };
    //     // assert_eq!(vec![0, 0, 0], slice);
    //
    //     allocation::deallocate(ptr, len);
    //
    //     let some_vec: Vec<u8> = vec![1_u8, 10_u8, 100_u8];
    //
    //     // the new vec should have the same pointer as the original allocation after we deallocate
    //     assert_eq!(ptr, some_vec.as_ptr() as Ptr);
    //
    //     let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };
    //
    //     // the same sized slice at the same pointer now looks like some_vec
    //     assert_eq!(slice.to_vec(), some_vec);
    // }

    // #[test]
    // fn allocation_ptr_round_trip() {
    //     let allocation: Allocation = [1, 2];
    //     let allocation_ptr: AllocationPtr = allocation.into();
    //     let guest_ptr: GuestPtr = allocation_ptr.as_guest_ptr();
    //
    //     // can peek without any deallocations
    //     assert_eq!(allocation_ptr.peek_allocation(), [1, 2],);
    //     assert_eq!(allocation_ptr.peek_allocation(), [1, 2],);
    //
    //     // can round trip back
    //     let returned_allocation: Allocation = allocation_ptr.into();
    //
    //     assert_eq!(returned_allocation, [1, 2]);
    //
    //     // round tripping above deallocates the original allocation
    //     // put something here to try and make sure memory doesn't stick around
    //     let _: Allocation = [3, 4];
    //     assert_ne!(
    //         AllocationPtr::from_guest_ptr(guest_ptr).peek_allocation(),
    //         [1, 2]
    //     );
    // }

    // #[test]
    // fn serialized_bytes_from_allocation_test() {
    //     let foo: Foo = Foo("foo".into());
    //     let foo_clone = foo.clone();
    //     let foo_sb: SerializedBytes = foo.try_into().unwrap();
    //     let foo_sb_clone = foo_sb.clone();
    //
    //     let ptr: AllocationPtr = foo_sb.into();
    //     let guest_ptr: GuestPtr = ptr.as_guest_ptr();
    //
    //     // the Allocation should get deallocated so this should not match
    //     // after the
    //     let unexpected_allocation: Allocation = ptr.peek_allocation();
    //
    //     // ownership of these bytes should be taken by SerializedBytes
    //     let inner_bytes: Vec<u8> = unsafe {
    //         std::slice::from_raw_parts(unexpected_allocation[0] as _, unexpected_allocation[1] as _)
    //     }
    //     .to_vec();
    //
    //     let recovered_foo_sb: SerializedBytes = ptr.into();
    //
    //     // the AllocationPtr's Allocation should be deallocated here
    //     assert_ne!(
    //         AllocationPtr::from_guest_ptr(guest_ptr).peek_allocation(),
    //         unexpected_allocation
    //     );
    //
    //     assert_eq!(foo_sb_clone, recovered_foo_sb);
    //
    //     let recovered_foo: Foo = recovered_foo_sb.try_into().unwrap();
    //
    //     let inner_bytes_2: Vec<u8> = unsafe {
    //         std::slice::from_raw_parts(unexpected_allocation[0] as _, unexpected_allocation[1] as _)
    //     }
    //     .to_vec();
    //
    //     // inner_bytes_2 should be nothing because inner_bytes was owned by SerializedBytes which
    //     // turned into a Foo
    //     assert_ne!(inner_bytes, inner_bytes_2,);
    //
    //     assert_eq!(foo_clone, recovered_foo);
    // }
}
