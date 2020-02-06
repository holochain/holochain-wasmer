// use common::memory::AllocationBytes;
use common::memory::AllocationPtr;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;
// use common::memory::ALLOCATION_ITEMS;
// use std::convert::TryInto;
use common::allocate::allocation_from_allocation_ptr;
use common::memory::Allocation;
use std::io::Read;
// use std::slice;
use byte_slice_cast::*;
use wasmer_runtime::Ctx;

pub fn copy_allocation_to_guest(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
    host_allocation_ptr: AllocationPtr,
) {
    let host_allocation: Allocation = allocation_from_allocation_ptr(host_allocation_ptr);

    let memory = ctx.memory(0);

    for (byte, cell) in host_allocation.as_byte_slice().bytes().zip(
        memory.view()[guest_allocation_ptr as _
            ..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
            .iter(),
    ) {
        cell.set(byte.unwrap())
    }
}

pub fn copy_string_to_guest(ctx: &mut Ctx, guest_ptr: Ptr, s: String) {
    let memory = ctx.memory(0);

    for (byte, cell) in s
        .bytes()
        .zip(memory.view()[guest_ptr as _..(guest_ptr + s.len() as Ptr) as _].iter())
    {
        cell.set(byte)
    }
}
