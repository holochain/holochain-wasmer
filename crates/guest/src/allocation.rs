use holochain_wasmer_common::*;

#[inline(always)]
/// Attempt to extract the length at the given guest_ptr.
//. Note that the guest_ptr could point at garbage and the "length prefix" would be garbage and
//. then some arbitrary memory would be referenced so not erroring does not imply safety.
pub fn length_prefix_at_guest_ptr(guest_ptr: GuestPtr) -> Len {
    let len_bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(guest_ptr as *const u8, core::mem::size_of::<Len>()) };
    u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
}

#[no_mangle]
#[inline(always)]
/// Allocate a length __plus a length prefix__ in bytes that won't be dropped by the allocator.
/// Return the pointer to it so a length prefix + bytes can be written to the allocation.
pub extern "C" fn __allocate(len: Len) -> GuestPtr {
    let dummy: Vec<u8> = Vec::with_capacity((len + core::mem::size_of::<Len>() as Len) as usize);
    let ptr = dummy.as_ptr() as GuestPtr;
    let _ = core::mem::ManuallyDrop::new(dummy);
    ptr
}

#[no_mangle]
#[inline(always)]
/// Free a length-prefixed allocation.
/// Needed because we leak memory every time we call `__allocate` and `write_bytes`.
pub extern "C" fn __deallocate(guest_ptr: GuestPtr) {
    // Failing to deallocate when requested is unrecoverable.
    let len = length_prefix_at_guest_ptr(guest_ptr) + core::mem::size_of::<Len>() as Len;
    let _: Vec<u8> =
        unsafe { Vec::from_raw_parts(guest_ptr as *mut u8, len as usize, len as usize) };
}

#[inline(always)]
/// Attempt to consume bytes out of a length-prefixed allocation at the given pointer position.
///
/// Consume in this context means take ownership of previously forgotten data.
///
/// This needs to work for bytes written into the guest from the host and for bytes written with
/// the write_bytes() function within the guest.
pub fn consume_bytes(guest_ptr: GuestPtr) -> Vec<u8> {
    // The Vec safety requirements are much stricter than a simple slice:
    //
    // - the pointer must have been generated with Vec/String on the same allocator
    //   - yes: the guest always creates its own GuestPtr for vector allocations, note that if we
    //     change the GuestPtr at all (e.g. to try and offset the length prefix) this will break,
    //     we must use the _exact_ GuestPtr previously allocated
    // - the pointer and T need the same size and alignment
    //   - yes: we can cast GuestPtr to *mut u8
    // - length needs to be compatible with capacity
    //   - yes: we can set length and capacity equal
    // - capacity needs to match capacity at the time of the pointer creation
    //   - yes: this is trickier, what we _want_ is a vector of just the payload bytes without the
    //     length prefix, but if we try to change either the capacity or the pointer to offset the
    //     prefix directly then we end up with MemoryOutOfBounds exceptions down the line
    // - @see https://doc.rust-lang.org/std/vec/struct.Vec.html#safety
    //
    // For example, this did _not_ work and leads to memory related panics down the line:
    // let v: Vec<u8> = Vec::from_raw_parts(
    //     (guest_ptr + std::mem::size_of::<Len>() as Len) as *mut u8,
    //     len as usize,
    //     (len + std::mem::size_of::<Len>() as Len) as usize,
    // );

    // We need the same length used to allocate the vector originally, so it includes the prefix
    let len = length_prefix_at_guest_ptr(guest_ptr) + core::mem::size_of::<Len>() as Len;
    let mut v: Vec<u8> = unsafe {
        Vec::from_raw_parts(
            // must match the pointer produced by the original allocation exactly
            guest_ptr as *mut u8,
            // this is the full length of the allocation as we want all the bytes
            len as usize,
            // must match the capacity set during the original allocation exactly
            len as usize,
        )
    };

    // This leads to an additional allocation for a new vector starting after the length prefix
    // the old vector will be dropped and cleaned up by the allocator after this call
    // the split off bytes will take ownership moving forward.
    //
    // Note that we could have tried to do something with std::slice::from_raw_parts() in this
    // function but we'd still need a new allocation at the point of slice.to_vec() and then
    // we'd need to manually free whatever the slice was pointing at.
    v.split_off(core::mem::size_of::<Len>())
}

#[inline(always)]
/// Attempt to write a slice of bytes into a length prefixed allocation.
///
/// This is identical to the following:
/// - host has some slice of bytes
/// - host calls __allocate with the slice length
/// - guest returns GuestPtr to the host
/// - host writes a length prefix and the slice bytes into the guest at GuestPtr location
/// - host hands the GuestPtr back to the guest
///
/// In this case everything happens within the guest and a GuestPtr is returned if successful.
///
/// This also leaks the written bytes, exactly like the above process.
///
/// This facilitates the guest handing a GuestPtr back to the host as the _return_ value of guest
/// functions so that the host can read the _output_ of guest logic from a length-prefixed pointer.
///
/// A good host will call __deallocate with the GuestPtr produced here once it has read the bytes
/// out of the guest, otherwise the bytes will be permanently leaked for the lifetime of the guest.
pub fn write_bytes(slice: &[u8]) -> GuestPtr {
    let len_bytes = slice.len().to_le_bytes();

    let v: Vec<u8> = len_bytes.iter().chain(slice.iter()).cloned().collect();
    let ptr: GuestPtr = v.as_ptr() as GuestPtr;
    let _ = core::mem::ManuallyDrop::new(v);
    ptr
}
