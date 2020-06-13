use holochain_wasmer_guest::*;

host_externs!(
    __debug
);

#[no_mangle]
pub extern "C" fn allocate_allocation_ptr_test(_: GuestPtr) -> GuestPtr {
    let some_ptr = 50 as GuestPtr;
    let some_len = 100 as Len;

    let slice = WasmSlice::from([some_ptr, some_len]);
    let allocation_ptr = AllocationPtr::from(slice);

    let restored_slice: WasmSlice = allocation_ptr.into();

    assert_eq!(
        [some_ptr, some_len],
        [restored_slice.ptr(), restored_slice.len()],
    );

    ret!(());
}

#[no_mangle]
pub extern "C" fn dellocate_test(_: GuestPtr) -> GuestPtr {
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
    assert_eq!(ptr, some_vec.as_ptr() as GuestPtr);

    let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };

    // the same sized slice at the same pointer now looks like some_vec
    assert_eq!(slice.to_vec(), some_vec);

    ret!(());
}

#[no_mangle]
pub extern "C" fn allocation_ptr_round_trip(_: GuestPtr) -> GuestPtr {
    let slice: WasmSlice = [1_u32, 2_u32].into();
    let allocation_ptr: AllocationPtr = slice.into();
    let guest_ptr: GuestPtr = allocation_ptr.as_guest_ptr();
    unsafe { __debug(guest_ptr); }
    unsafe { __debug(std::slice::from_raw_parts(guest_ptr as _, 2)[0]); }
    unsafe { __debug(std::slice::from_raw_parts(guest_ptr as _, 2)[1]); }

    // can round trip back
    let returned_slice: WasmSlice = allocation_ptr.into();

    assert_eq!([returned_slice.ptr(), returned_slice.len()], [1_u32, 2_u32]);

    // round tripping above deallocates the original allocation
    // put something here to try and make sure memory doesn't stick around
    let _a: WasmSlice = [3, 4].into();
    let _b: WasmSlice = [3, 4].into();
    let _c: WasmSlice = [3, 4].into();

    unsafe { __debug(guest_ptr); }
    unsafe { __debug(std::slice::from_raw_parts(guest_ptr as _, 2)[0]); }
    unsafe { __debug(std::slice::from_raw_parts(guest_ptr as _, 2)[1]); }

    // assert_eq!(1_u32, guest_ptr as *const u32 as u32);
    // println!("{}", guest_ptr as *const u32 as u32);

    // assert_ne!(
    //     AllocationPtr::from_guest_ptr(guest_ptr).peek_allocation(),
    //     [1, 2]
    // );

    ret!(());
}

#[cfg(test)]
pub mod tests {

    // // use crate::allocation;
    // // use crate::allocation::Allocation;
    // // use crate::*;
    // use super::AllocationPtr;
    // use holochain_serialized_bytes::prelude::*;
    // use holochain_wasmer_common::slice::WasmSlice;
    // use holochain_wasmer_common::GuestPtr;
    // use holochain_wasmer_common::Len;
    //
    // #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, SerializedBytes)]
    // struct Foo(String);
    //

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
