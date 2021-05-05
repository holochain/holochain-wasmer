use holochain_wasmer_common::*;

#[no_mangle]
#[inline(always)]
/// Allocate bytes that won't be dropped by the allocator.
/// Return the pointer to the leaked allocation so the host can write to it.
pub extern "C" fn __allocate(len: Len) -> GuestPtr {
    let dummy: Vec<u8> = Vec::with_capacity((len + core::mem::size_of::<Len>() as Len) as usize);
    let ptr = dummy.as_ptr() as GuestPtr;
    let _ = core::mem::ManuallyDrop::new(dummy);
    ptr
}

#[no_mangle]
#[inline(always)]
/// Free an allocation.
/// Needed because we leak memory every time we call `__allocate` and `write_bytes`.
pub extern "C" fn __deallocate(guest_ptr: GuestPtr, len: Len) {
    let _: Vec<u8> =
        unsafe { Vec::from_raw_parts(guest_ptr as *mut u8, len as usize, len as usize) };
}

#[inline(always)]
/// Attempt to consume bytes from a known guest_ptr and len.
///
/// Consume in this context means take ownership of previously forgotten data.
///
/// This needs to work for bytes written into the guest from the host and for bytes written with
/// the write_bytes() function within the guest.
pub fn consume_bytes(guest_ptr: GuestPtr, len: Len) -> Vec<u8> {
    unsafe {
        Vec::from_raw_parts(
            // must match the pointer produced by the original allocation exactly
            guest_ptr as *mut u8,
            // this is the full length of the allocation as we want all the bytes
            len as usize,
            // must match the capacity set during the original allocation exactly
            len as usize,
        )
    }
}

#[inline(always)]
/// Attempt to write a slice of bytes.
///
/// This is identical to the following:
/// - host has some slice of bytes
/// - host calls __allocate with the slice length
/// - guest returns GuestPtr to the host
/// - host writes the bytes into the guest at GuestPtr location
/// - host hands the GuestPtr back to the guest
///
/// In this case everything happens within the guest and a GuestPtr is returned if successful.
///
/// This also leaks the written bytes, exactly like the above process.
///
/// This facilitates the guest handing a GuestPtr back to the host as the _return_ value of guest
/// functions so that the host can read the _output_ of guest logic from a pointer.
///
/// The host MUST ensure either __deallocate is called or the entire wasm memory is dropped.
pub fn write_bytes(slice: &[u8]) -> GuestPtr {
    let v = slice.to_vec();
    let ptr: GuestPtr = v.as_ptr() as GuestPtr;
    let _ = core::mem::ManuallyDrop::new(v);
    ptr
}
