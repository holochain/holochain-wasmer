use byte_slice_cast::AsByteSlice;
use common::allocation;
use common::allocation::Allocation;
use common::allocation::ALLOCATION_BYTES_ITEMS;
use common::AllocationPtr;
use common::Ptr;
use std::io::Read;
use wasmer_runtime::Ctx;

pub fn write_to_guest(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
    host_allocation_ptr: AllocationPtr,
) {
    let host_allocation: Allocation = allocation::from_allocation_ptr(host_allocation_ptr);

    let memory = ctx.memory(0);

    for (byte, cell) in host_allocation.as_byte_slice().bytes().zip(
        memory.view()[guest_allocation_ptr as _
            ..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
            .iter(),
    ) {
        // expect here because:
        // - on the host side rust backtraces work properly
        // - failing to write to pre-allocated memory should never happen
        // - results are not FFI safe so not compatible with wasm imports
        cell.set(byte.expect("a byte did not exist while writing to guest"));
    }
}
