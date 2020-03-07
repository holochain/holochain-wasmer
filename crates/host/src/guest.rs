use crate::allocation;
use crate::bytes;
use crate::*;
use byte_slice_cast::AsSliceOf;
use holochain_wasmer_common::JsonError;
use holochain_wasmer_common::JsonString;
use std::convert::TryFrom;
use std::convert::TryInto;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

pub fn write_bytes(ctx: &mut Ctx, guest_ptr: Ptr, bytes: Vec<u8>) {
    let memory = ctx.memory(0);

    for (byte, cell) in bytes
        .iter()
        .zip(memory.view()[guest_ptr as _..(guest_ptr + bytes.len() as Ptr) as _].iter())
    {
        cell.set(byte.to_owned());
    }
}

pub fn bytes_from_allocation_ptr(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<Vec<u8>, WasmError> {
    let view: MemoryView<u8> = ctx.memory(0).view();
    let bytes_vec: Vec<u8> = view[guest_allocation_ptr as _
        ..(guest_allocation_ptr + allocation::ALLOCATION_BYTES_ITEMS as Ptr) as _]
        .iter()
        .map(|cell| cell.get())
        .collect();
    let guest_allocation: allocation::Allocation = bytes_vec.as_slice_of::<u64>()?.try_into()?;

    Ok(
        view[guest_allocation[0] as usize..(guest_allocation[0] + guest_allocation[1]) as usize]
            .iter()
            .map(|cell| cell.get())
            .collect(),
    )
}

pub fn from_allocation_ptr<O: TryFrom<JsonString>>(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<O, WasmError>
where
    O::Error: Into<String>,
{
    let bytes = bytes_from_allocation_ptr(ctx, guest_allocation_ptr)?;
    let json = JsonString::from_bytes(bytes);
    match json.try_into() {
        Ok(v) => Ok(v),
        Err(e) => Err(WasmError::GuestResultHandling(e.into())),
    }
}

/// host calling guest for the function named `call` with the given `payload` in a vector of bytes
/// result is either a vector of bytes from the guest found at the location of the returned guest
/// allocation pointer or a wasm error
pub fn call_bytes(
    instance: &mut Instance,
    call: &str,
    payload: Vec<u8>,
) -> Result<Vec<u8>, WasmError> {
    let host_allocation_ptr = bytes::to_allocation_ptr(payload);

    // this requires that the guest exported function being called knows what to do with a
    // host allocation pointer
    let guest_allocation_ptr = match instance
        .call(call, &[Value::I64(host_allocation_ptr.try_into()?)])
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    Ok(crate::guest::bytes_from_allocation_ptr(
        instance.context_mut(),
        guest_allocation_ptr,
    )?)
}

/// convenience wrapper around call_bytes to handling input and output of any struct that:
/// - is commonly defined in both the host and guest (e.g. shared in a common crate)
/// - implements standard JsonString round-tripping (e.g. DefaultJson)
pub fn call<I: Into<JsonString>, O: TryFrom<JsonString, Error = JsonError>>(
    instance: &mut Instance,
    call: &str,
    jsonable: I,
) -> Result<O, WasmError> {
    let json: JsonString = jsonable.into();
    let bytes = json.to_bytes();
    let result_bytes = call_bytes(instance, call, bytes)?;
    let result_json = JsonString::from_bytes(result_bytes);
    let wasm_result: WasmResult = match result_json.try_into() {
        Ok(v) => v,
        Err(e) => return Err(WasmError::GuestResultHandling(e.into())),
    };
    match wasm_result {
        WasmResult::Ok(json) => match json.try_into() {
            Ok(v) => Ok(v),
            Err(e) => Err(WasmError::GuestResultHandling(e.into())),
        },
        WasmResult::Err(wasm_error) => return Err(wasm_error),
    }
}
