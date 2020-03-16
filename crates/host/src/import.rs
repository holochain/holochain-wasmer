use crate::allocation;
use crate::guest;
use crate::serialized_bytes;
use crate::*;
use byte_slice_cast::AsByteSlice;
use std::io::Read;
use wasmer_runtime::Ctx;
use holochain_wasmer_common::allocation::deallocate_from_allocation_ptr;

/// import an allocation from the host to the guest
/// - the guest allocation pointer must be preallocated
/// - the host allocation pointer must point to a valid allocation
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

    // note that we do NOT deallocate here as the host allocation_ptr needs to still be valid
    // when __import_bytes pulls the actual data across
    // the __import_bytes function deallocates both the allocation and the bytes it points to on
    // the host side
}

/// import bytes from the host allocation pointer to the guest bytes pointer
/// - the host allocation pointer must point to the allocation for the bytes to copy
/// - the guest bytes pointer must point to preallocated space with the correct length
pub fn __import_bytes(ctx: &mut Ctx, host_allocation_ptr: AllocationPtr, guest_bytes_ptr: Ptr) {
    let serialized_bytes: SerializedBytes =
        serialized_bytes::from_allocation_ptr(host_allocation_ptr);
    guest::write_bytes(ctx, guest_bytes_ptr, serialized_bytes);

    // deallocate the host-side allocation now that the guest holds it and all the bytes
    deallocate_from_allocation_ptr(host_allocation_ptr);
}
