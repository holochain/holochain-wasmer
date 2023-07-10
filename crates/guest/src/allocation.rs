/// Allocate bytes that won't be dropped by the allocator.
/// Return the pointer to the leaked allocation so the host can write to it.
#[no_mangle]
#[inline(always)]
pub extern "C" fn __allocate(len: usize) -> usize {
    write_bytes(Vec::with_capacity(len))
}

/// Free an allocation.
/// Needed because we leak memory every time we call `__allocate` and `write_bytes`.
#[no_mangle]
#[inline(always)]
pub extern "C" fn __deallocate(guest_ptr: usize, len: usize) {
    let _ = consume_bytes(guest_ptr, len);
}

/// Attempt to consume bytes from a known `guest_ptr` and `len`.
///
/// Consume in this context means take ownership of previously forgotten data.
///
/// This needs to work for bytes written into the guest from the host and for bytes written with
/// the `write_bytes` function within the guest.
#[inline(always)]
pub fn consume_bytes(guest_ptr: usize, len: usize) -> Vec<u8> {
    // This must be a Vec and not only a slice, because slices will fail to
    // deallocate memory properly when dropped.
    // Assumes length and capacity are the same, which is true if `__allocate` is
    // used to allocate memory for the vector.
    unsafe { std::vec::Vec::from_raw_parts(guest_ptr as *mut u8, len, len) }
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
pub fn write_bytes(v: Vec<u8>) -> usize {
    // This *const u8 cast to u32 is safe and the only way to get a raw pointer as a u32 afaik.
    // > e has type *T and U is a numeric type, while T: Sized; ptr-addr-cast
    // https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/first-edition/casting-between-types.html#pointer-casts
    v.leak().as_ptr() as usize
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn round_trip_5_5() {
        let bytes_round = consume_bytes(write_bytes(vec![1, 2, 3, 4, 5]), 5);

        dbg!(&bytes_round);
    }

    fn _round_trip_allocation(bytes: Vec<u8>) {
        let bytes_round = consume_bytes(write_bytes(bytes.clone()), bytes.len());

        assert_eq!(bytes, bytes_round);
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn round_trip_allocation(bytes: Vec<u8>) {
        _round_trip_allocation(bytes);
    }

    #[test]
    fn some_round_trip_allocation() {
        _round_trip_allocation(vec![1, 2, 3]);
    }

    fn _alloc_dealloc(len: usize) {
        __deallocate(__allocate(len), len);
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn alloc_dealloc(len: usize) {
        _alloc_dealloc(len);
    }

    #[test]
    fn some_alloc_dealloc() {
        _alloc_dealloc(1_000_000_000_usize);
    }
}
