use byte_slice_cast::*;
use common::allocate::allocation_from_allocation_ptr;
use common::error::Error;
use common::memory::Allocation;
use common::memory::AllocationPtr;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;
use std::io::Read;
use wasmer_runtime::Ctx;

pub fn copy_allocation_to_guest(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
    host_allocation_ptr: AllocationPtr,
) -> Result<(), Error> {
    let mut failed = false;
    let host_allocation: Allocation = allocation_from_allocation_ptr(host_allocation_ptr);

    let memory = ctx.memory(0);

    for (byte, cell) in host_allocation.as_byte_slice().bytes().zip(
        memory.view()[guest_allocation_ptr as _
            ..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
            .iter(),
    ) {
        match byte {
            Ok(b) => cell.set(b),
            Err(_) => failed = true,
        }
    }

    if failed {
        Err(Error::Memory)
    } else {
        Ok(())
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
