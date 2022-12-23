use holochain_wasmer_common::*;

/// Allocate bytes that won't be dropped by the allocator.
/// Return the pointer to the leaked allocation so the host can write to it.
#[no_mangle]
#[inline(always)]
pub extern "C" fn __allocate(len: Len) -> GuestPtr {
    write_bytes(Vec::with_capacity(
        // If `usize` is smaller than `u32` the host cannot support that so we
        // panic/unwrap.
        len.try_into().unwrap(),
    ))
}

/// Free an allocation.
/// Needed because we leak memory every time we call `__allocate` and `write_bytes`.
#[no_mangle]
#[inline(always)]
pub extern "C" fn __deallocate(guest_ptr: GuestPtr, len: Len) {
    let _ = consume_bytes(guest_ptr, len);
}

/// Attempt to consume bytes from a known `guest_ptr` and `len`.
///
/// Consume in this context means take ownership of previously forgotten data.
///
/// This needs to work for bytes written into the guest from the host and for bytes written with
/// the `write_bytes` function within the guest.
#[inline(always)]
pub fn consume_bytes(guest_ptr: GuestPtr, len: Len) -> Vec<u8> {
    // If `usize` is smaller than `u32`, the host cannot support that so we
    // panic/unwrap.
    let len_usize: usize = len.try_into().unwrap();
    // This must be a Vec and not only a slice, because slices will fail to
    // deallocate memory properly when dropped.
    // Assumes length and capacity are the same, which is true if `__allocate` is
    // used to allocate memory for the vector.
    unsafe { std::vec::Vec::from_raw_parts(guest_ptr as *mut u8, len_usize, len_usize) }
}

/// Given an owned vector of bytes, leaks it and returns a pointer the host can
/// use to read the bytes. This does NOT handle the length of the bytes, so the
/// guest will need to track the length separately from leaking the vector.
///
/// This facilitates the guest handing a `GuestPtr` back to the host as the return value of guest
/// functions so that the host can read the output of guest logic from a pointer.
///
/// The host MUST ensure either `__deallocate` is called or the entire wasm memory is dropped.
/// If the host fails to tell the guest where and how many bytes to deallocate, then this leak
/// becomes permanent to the guest.
#[inline(always)]
pub fn write_bytes(v: Vec<u8>) -> GuestPtr {
    // This *const u8 cast to u32 is safe and the only way to get a raw pointer as a u32 afaik.
    // > e has type *T and U is a numeric type, while T: Sized; ptr-addr-cast
    // https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/first-edition/casting-between-types.html#pointer-casts
    v.leak().as_ptr() as u32
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test_fuzz::test_fuzz]
    fn round_trip(bytes: Vec<u8>) {
        let bytes_round = consume_bytes(write_bytes(bytes.clone()), bytes.len());

        assert_eq!(bytes, bytes_round);
    }
}
