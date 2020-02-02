use std::slice;
use wasmer_runtime::Ctx;
use common::memory::AllocationBytes;
use common::memory::ALLOCATION_BYTES_ITEMS;
use std::convert::TryInto;
use std::io::Read;
use common::memory::AllocationPtr;
use common::memory::ALLOCATION_ITEMS;
use common::memory::Ptr;

pub fn copy_allocation_to_guest(ctx: &mut Ctx, guest_ptr: Ptr, allocation_ptr: AllocationPtr) {
    let memory = ctx.memory(0);
    let slice: AllocationBytes = unsafe { slice::from_raw_parts(allocation_ptr as _, ALLOCATION_BYTES_ITEMS) }.try_into().unwrap();

    for (byte, cell) in slice
        .bytes()
        .zip(
            memory.view()
            [guest_ptr as _..(guest_ptr + ALLOCATION_ITEMS as Ptr) as _].iter())
    {
            cell.set(byte.unwrap())
    };
}

pub fn copy_string_to_guest(ctx: &mut Ctx, guest_ptr: Ptr, s: String) {
    let memory = ctx.memory(0);

    for (byte, cell) in s
        .bytes()
        .zip(memory.view()[guest_ptr as _..(guest_ptr + s.len() as Ptr) as _].iter()) {
            cell.set(byte)
    };
}
