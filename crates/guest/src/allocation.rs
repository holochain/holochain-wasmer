// use holochain_wasmer_common::slice;
use byte_slice_cast::AsByteSlice;
use byte_slice_cast::AsSliceOf;
use holochain_wasmer_common::*;
use std::mem;

fn read_length(guest_ptr: GuestPtr) -> Result<Len, WasmError> {
    let len_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(guest_ptr as _, std::mem::size_of::<Len>()) };
    let len: Len = len_bytes.as_slice_of::<Len>()?[0];
    Ok(len)
}

#[no_mangle]
/// allocate a length of bytes that won't be dropped by the allocator
/// return the pointer to it so bytes can be written to the allocation
pub extern "C" fn __allocate(len: Len) -> GuestPtr {
    let dummy: Vec<u8> = Vec::with_capacity(allocation_length(len) as _);
    let ptr = dummy.as_ptr() as GuestPtr;
    mem::ManuallyDrop::new(dummy);
    ptr
}

/// restore an allocation so that it is dropped immediately
/// this needs to be called on anything allocated above as the allocator
/// will never free the memory otherwise
#[no_mangle]
pub extern "C" fn __deallocate(guest_ptr: GuestPtr) {
    let len = allocation_length(read_length(guest_ptr).unwrap());
    let _: Vec<u8> = unsafe { Vec::from_raw_parts(guest_ptr as _, len as _, len as _) };
}

pub fn read_bytes(guest_ptr: GuestPtr) -> Result<Vec<u8>, WasmError> {
    let len = read_length(guest_ptr)?;

    Ok(unsafe {
        Vec::from_raw_parts(
            (guest_ptr + std::mem::size_of::<Len>() as Len) as _,
            len as _,
            len as _,
        )
    })
}

pub fn write_bytes(slice: &[u8]) -> Result<GuestPtr, WasmError> {
    let len = slice.len() as Len;
    let len_array: [Len; 1] = [len];
    let len_bytes: &[u8] = len_array.as_byte_slice();

    let v: Vec<u8> = len_bytes.iter().chain(slice.iter()).cloned().collect();
    let ptr: GuestPtr = v.as_ptr() as GuestPtr;
    mem::ManuallyDrop::new(v);
    Ok(ptr)
}

// /// AllocationPtr wraps a ptr that is used to pass the location of an Allocation
// /// between the host and guest (in either direction).
// /// The AllocationPtr intentionally does not implement Clone
// /// The From<Allocation> and Into<Allocation> round trip handles manually allocating
// /// and deallocating an internal vector that is shared across host/guest
// /// If the AllocationPtr was to be cloned the shared vector could be allocated and
// /// deallocated in an undefined way
// pub struct AllocationPtr(GuestPtr);
//
// impl AllocationPtr {
//     /// normally we don't want to expose the inner Ptr because cloning or reusing it
//     /// can lead to bad allocation and deallocation
//     /// in the case that we pass the inner Ptr over to the host we do need to access it
//     /// this function is just like From<AllocationPtr> for Ptr but renamed to make it
//     /// clear that it has only one valid use-case
//     pub fn as_guest_ptr(&self) -> GuestPtr {
//         self.0
//     }
//
//     pub fn from_guest_ptr(guest_ptr: GuestPtr) -> Self {
//         Self(guest_ptr)
//     }
// }

// / given an WasmSlice returns an AllocationPtr pointer to it
// / the WasmSlice is internally converted to a Vec<u8> that requires a manual drop
// /
// / this is analagous to Box::into_raw()
// /
// / i.e. the allocation vec is not dropped when it goes out of scope
// / the pointer returned is to this new vector and the originally passed allocation is handled by
// / the rust allocater as per normal ownership rules
// / this allows us to pass the AllocationPtr across the host/guest boundary in either direction and
// / have the WasmFatPtr bytes remain in memory until the AllocationPtr recipient is ready to have
// / them copied in.
// / the From<AllocationPtr> function does the inverse and internally deallocates the allocation
// / vector that is created here.
// / to avoid memory leaks, every WasmSlice must fully round-trip through these two functions.
// impl From<slice::WasmSlice> for AllocationPtr {
//     fn from(slice: slice::WasmSlice) -> Self {
//         // the allocation must exist as a vector or it will be dropped
//         // slices drop even with ManuallyDrop
//         let allocation_vec: Vec<GuestPtr> = vec![slice.ptr(), slice.len()];
//         let allocation_ptr = allocation_vec.as_ptr() as GuestPtr;
//         mem::ManuallyDrop::new(allocation_vec);
//         AllocationPtr(allocation_ptr)
//     }
// }

// / inverse of From<fat_ptr::WasmFatPtr> for AllocationPtr
// /
// / unleaks the previously leaked memory
// /
// / @see From<WasmSlice> for AllocationPtr
// impl From<AllocationPtr> for slice::WasmSlice {
//     fn from(allocation_ptr: AllocationPtr) -> Self {
//         // this is the inverse of From<WasmSlice>
//         // it deallocates what From<WasmSlice> did an unsafe allocation for
//         let allocation_vec: Vec<GuestPtr> = unsafe {
//             Vec::from_raw_parts(
//                 allocation_ptr.as_guest_ptr() as _,
//                 slice::WASM_SLICE_ITEMS,
//                 slice::WASM_SLICE_ITEMS,
//             )
//         };
//         // this is a new allocation that will be handled by the rust allocator
//         [allocation_vec[0], allocation_vec[1]].into()
//     }
// }

// impl From<AllocationPtr> for SerializedBytes {
//     fn from(allocation_ptr: AllocationPtr) -> SerializedBytes {
//         let slice = slice::WasmSlice::from(allocation_ptr);
//         let b: Vec<u8> =
//             unsafe { Vec::from_raw_parts(slice.ptr() as _, slice.len() as _, slice.len() as _) };
//         SerializedBytes::from(UnsafeBytes::from(b))
//     }
// }

// impl From<SerializedBytes> for AllocationPtr {
//     fn from(sb: SerializedBytes) -> AllocationPtr {
//         let bytes: Vec<u8> = UnsafeBytes::from(sb).into();
//         let bytes_ptr = bytes.as_ptr() as GuestPtr;
//         let bytes_len = bytes.len() as Len;
//         std::mem::ManuallyDrop::new(bytes);
//         // move through a slice to leak correctly
//         let slice: slice::WasmSlice = [bytes_ptr, bytes_len].into();
//         AllocationPtr::from(slice)
//     }
// }
