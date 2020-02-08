use byte_slice_cast::AsSliceOf;
use common::allocation::Allocation;
use common::allocation::ALLOCATION_BYTES_ITEMS;
use common::bytes;
use common::error::Error;
use common::AllocationPtr;
use common::Ptr;
use std::convert::TryInto;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;

pub fn write_to_guest(ctx: &mut Ctx, guest_ptr: Ptr, bytes: Vec<u8>) {
    let memory = ctx.memory(0);

    for (byte, cell) in bytes
        .iter()
        .zip(memory.view()[guest_ptr as _..(guest_ptr + bytes.len() as Ptr) as _].iter())
    {
        cell.set(byte.to_owned());
    }
}

pub fn write_to_guest_using_host_allocation_ptr(
    ctx: &mut Ctx,
    host_allocation_ptr: AllocationPtr,
    guest_bytes_ptr: Ptr,
) {
    let bytes = bytes::from_allocation_ptr(host_allocation_ptr);
    write_to_guest(ctx, guest_bytes_ptr, bytes);
}

pub fn read_from_guest(ctx: &mut Ctx, ptr: Ptr, len: Ptr) -> Vec<u8> {
    let memory = ctx.memory(0);
    let vec: Vec<u8> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    // Ok(std::str::from_utf8(&str_vec)?.into())
    vec
}

pub fn read_from_guest_using_allocation_ptr(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<Vec<u8>, Error> {
    let view: MemoryView<u8> = ctx.memory(0).view();
    let bytes_vec: Vec<u8> = view
        [guest_allocation_ptr as _..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
        .iter()
        .map(|cell| cell.get())
        .collect();
    let guest_allocation: Allocation = bytes_vec
        .as_slice_of::<u64>()?
        .try_into()
        .expect("wrong number of array elements");

    Ok(read_from_guest(
        ctx,
        guest_allocation[0],
        guest_allocation[1],
    ))
}
