use crate::guest;
use crate::prelude::*;
use byte_slice_cast::AsByteSlice;
// use std::io::Read;
use byte_slice_cast::AsSliceOf;
use wasmer_runtime::Ctx;

/// import an allocation from the host to the guest
/// - the guest allocation pointer must be preallocated
/// - the host allocation pointer must point to a valid allocation
pub fn __import_allocation(
    ctx: &mut Ctx,
    guest_allocation_ptr: RemotePtr,
    host_allocation_ptr: RemotePtr,
) {
    // we do NOT want to deallocate the Allocation for the AllocationPtr so we peek instead of into
    // this is because __import_bytes needs to use the exact same Allocation in memory
    let host_allocation: allocation::Allocation =
        AllocationPtr::from_remote_ptr(host_allocation_ptr).peek_allocation();

    guest::write_slice(
        ctx,
        guest_allocation_ptr,
        host_allocation.as_byte_slice().as_slice_of::<u8>().unwrap(),
    );

    // let memory = ctx.memory(0);
    //
    // for (byte, cell) in host_allocation.as_byte_slice().bytes().zip(
    //     memory.view()[guest_allocation_ptr as _
    //         ..(guest_allocation_ptr + allocation::ALLOCATION_BYTES_ITEMS as Ptr) as _]
    //         .iter(),
    // ) {
    //     // expect here because:
    //     // - on the host side rust backtraces work properly
    //     // - failing to write to pre-allocated memory should never happen
    //     // - results are not FFI safe so not compatible with wasm imports
    //     cell.set(byte.expect("a byte did not exist while writing to guest"));
    // }
}

/// import bytes from the host allocation pointer to the guest bytes pointer
/// - the host allocation pointer must point to the allocation for the bytes to copy
/// - the guest bytes pointer must point to preallocated space with the correct length
pub fn __import_bytes(ctx: &mut Ctx, host_allocation_ptr: RemotePtr, guest_bytes_ptr: RemotePtr) {
    let serialized_bytes: SerializedBytes =
        AllocationPtr::from_remote_ptr(host_allocation_ptr).into();
    guest::write_slice(ctx, guest_bytes_ptr, serialized_bytes.bytes());
}
