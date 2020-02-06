pub mod allocate;

// Import the Filesystem so we can read our .wasm file
use crate::allocate::copy_string_to_guest;
use byte_slice_cast::AsSliceOf;
use common::allocate::string_allocation_ptr;
use common::allocate::string_from_allocation_ptr;
use common::error::Error;
use common::memory::Allocation;
use common::memory::AllocationPtr;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;
use std::convert::TryInto;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

pub fn read_guest_bytes(ctx: &Ctx, ptr: Ptr, len: Ptr) -> Vec<u8> {
    let memory = ctx.memory(0);
    let vec: Vec<u8> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    // Ok(std::str::from_utf8(&str_vec)?.into())
    vec
}

pub fn read_guest_bytes_from_allocation_ptr(
    ctx: &Ctx,
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

    Ok(read_guest_bytes(
        ctx,
        guest_allocation[0],
        guest_allocation[1],
    ))
}

pub fn host_copy_string(ctx: &mut Ctx, host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr) {
    let s = string_from_allocation_ptr(host_allocation_ptr);
    copy_string_to_guest(ctx, guest_string_ptr, s);
}

pub fn guest_call(instance: &Instance, call: &str, payload: &String) -> Result<String, Error> {
    let starter_string_allocation_ptr = string_allocation_ptr(payload.clone());

    let guest_allocation_ptr = match instance
        .call(
            call,
            &[Value::I64(starter_string_allocation_ptr.try_into()?)],
        )
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    Ok(std::str::from_utf8(&read_guest_bytes_from_allocation_ptr(
        &instance.context(),
        guest_allocation_ptr,
    )?)?
    .into())
}
