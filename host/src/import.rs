use crate::allocation;
use crate::bytes;
use crate::guest;
use crate::*;
use byte_slice_cast::AsByteSlice;
use std::io::Read;
use wasmer_runtime::Ctx;

pub fn __import_allocation(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
    host_allocation_ptr: AllocationPtr,
) {
    let host_allocation: allocation::Allocation =
        allocation::from_allocation_ptr(host_allocation_ptr);

    let memory = ctx.memory(0);

    for (byte, cell) in host_allocation.as_byte_slice().bytes().zip(
        memory.view()[guest_allocation_ptr as _
            ..(guest_allocation_ptr + allocation::ALLOCATION_BYTES_ITEMS as Ptr) as _]
            .iter(),
    ) {
        // expect here because:
        // - on the host side rust backtraces work properly
        // - failing to write to pre-allocated memory should never happen
        // - results are not FFI safe so not compatible with wasm imports
        cell.set(byte.expect("a byte did not exist while writing to guest"));
    }
}

pub fn __import_bytes(ctx: &mut Ctx, host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr) {
    let bytes = bytes::from_allocation_ptr(host_allocation_ptr);
    guest::write_bytes(ctx, guest_bytes_ptr, bytes);
}
