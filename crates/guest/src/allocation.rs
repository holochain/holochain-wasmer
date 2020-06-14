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
        std::slice::from_raw_parts(
            (guest_ptr + std::mem::size_of::<Len>() as Len) as _,
            len as _,
        )
    }
    .to_vec())
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
