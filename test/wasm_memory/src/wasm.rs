use holochain_wasmer_guest::*;

host_externs!(
    __debug,
    __pages
);

#[no_mangle]
pub extern "C" fn bytes_round_trip(_: GuestPtr) -> GuestPtr {

    let mut old_pages: WasmSize = unsafe { __pages(0) };
    let mut current_pages: WasmSize = old_pages;

    // thrash this more times than there are bytes in a wasm page so that if even one byte leaks
    // we will see it in the page count
    for i in 0..100_000 {

        // thrash a bunch of little chunks of bytes so that we can be reasonably sure the
        // allocations are in the correct position and not overlapping
        let bytes: Vec<[u8; 5]> = std::iter::repeat([ 1, 2, 3, 4, 5 ]).take(100).collect();

        let ptrs: Vec<GuestPtr> = bytes.iter().map(|b| {
            allocation::write_bytes(b).unwrap()
        }).collect();

        for i in 0..ptrs.len() {
            // the length prefix needs to be correct
            assert_eq!(
                allocation::length_prefix_at_guest_ptr(ptrs[i]).unwrap(),
                bytes[i].len() as Len,
            );

            // consuming the bytes should give a vector of the same bytes as the original bytes
            assert_eq!(
                bytes[i].to_vec(),
                allocation::consume_bytes(ptrs[i]).unwrap(),
            );
        };

        // if we forget to deallocate properly then the number of allocated pages will grow
        old_pages = current_pages;
        current_pages = unsafe { __pages(0) };
        if i > 0 {
            assert_eq!(old_pages, current_pages);
        }
    }

    ret!(());
}
