use crate::allocation;
use crate::serialized_bytes;
use crate::*;
use byte_slice_cast::AsSliceOf;
use holochain_serialized_bytes::prelude::*;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

pub fn write_bytes(ctx: &mut Ctx, guest_ptr: Ptr, serialized_bytes: SerializedBytes) {
    let memory = ctx.memory(0);

    for (byte, cell) in serialized_bytes.bytes().iter().zip(
        memory.view()[guest_ptr as _..(guest_ptr + serialized_bytes.bytes().len() as Ptr) as _]
            .iter(),
    ) {
        cell.set(byte.to_owned());
    }
}

pub fn serialized_bytes_from_allocation_ptr(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<SerializedBytes, WasmError> {
    let view: MemoryView<u8> = ctx.memory(0).view();
    let bytes_vec: Vec<u8> = view[guest_allocation_ptr as _
        ..(guest_allocation_ptr + allocation::ALLOCATION_BYTES_ITEMS as Ptr) as _]
        .iter()
        .map(|cell| cell.get())
        .collect();
    let guest_allocation: allocation::Allocation = bytes_vec.as_slice_of::<u64>()?.try_into()?;

    Ok(SerializedBytes::from(UnsafeBytes::from(
        view[guest_allocation[0] as usize..(guest_allocation[0] + guest_allocation[1]) as usize]
            .iter()
            .map(|cell| cell.get())
            .collect::<Vec<u8>>(),
    )))
}

pub fn from_allocation_ptr<O: TryFrom<SerializedBytes>>(
    ctx: &mut Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<O, WasmError>
where
    O::Error: Into<String>,
{
    let serialized_bytes: SerializedBytes =
        serialized_bytes_from_allocation_ptr(ctx, guest_allocation_ptr)?;
    match serialized_bytes.try_into() {
        Ok(v) => Ok(v),
        Err(e) => Err(WasmError::GuestResultHandling(e.into())),
    }
}

/// host calling guest for the function named `call` with the given `payload` in a vector of bytes
/// result is either a vector of bytes from the guest found at the location of the returned guest
/// allocation pointer or a wasm error
fn call_inner(
    instance: &mut Instance,
    call: &str,
    payload: SerializedBytes,
) -> Result<SerializedBytes, WasmError> {
    let host_allocation_ptr = serialized_bytes::to_allocation_ptr(payload);

    // this requires that the guest exported function being called knows what to do with a
    // host allocation pointer
    let guest_allocation_ptr = match instance
        .call(call, &[Value::I64(host_allocation_ptr.try_into()?)])
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    Ok(crate::guest::serialized_bytes_from_allocation_ptr(
        instance.context_mut(),
        guest_allocation_ptr,
    )?)
}

/// convenience wrapper around call_bytes to handling input and output of any struct that:
/// - is commonly defined in both the host and guest (e.g. shared in a common crate)
/// - implements standard JsonString round-tripping (e.g. DefaultJson)
pub fn call<
    I: TryInto<SerializedBytes, Error = SerializedBytesError>,
    O: TryFrom<SerializedBytes, Error = SerializedBytesError>,
>(
    instance: &mut Instance,
    call: &str,
    serializable: I,
) -> Result<O, WasmError> {
    let serialized_bytes: SerializedBytes = match serializable.try_into() {
        Ok(v) => v,
        Err(e) => return Err(WasmError::GuestResultHandling(e.into())),
    };
    let result_serialized_bytes: SerializedBytes = call_inner(instance, call, serialized_bytes)?;
    let wasm_result: WasmResult = match result_serialized_bytes.try_into() {
        Ok(v) => v,
        Err(e) => return Err(WasmError::GuestResultHandling(e.into())),
    };
    match wasm_result {
        WasmResult::Ok(inner_serialized_bytes) => match inner_serialized_bytes.try_into() {
            Ok(v) => Ok(v),
            Err(e) => Err(WasmError::GuestResultHandling(e.into())),
        },
        WasmResult::Err(wasm_error) => return Err(wasm_error),
    }
}
