use crate::allocation;
use crate::*;
use byte_slice_cast::AsSliceOf;
use holochain_serialized_bytes::prelude::*;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

pub fn write_bytes(ctx: &mut Ctx, guest_ptr: RemotePtr, serialized_bytes: SerializedBytes) {
    let memory = ctx.memory(0);

    for (byte, cell) in serialized_bytes.bytes().iter().zip(
        memory.view()[guest_ptr as _..(guest_ptr + serialized_bytes.bytes().len() as Ptr) as _]
            .iter(),
    ) {
        cell.set(byte.to_owned());
    }
}

pub fn serialized_bytes_from_guest_ptr(
    ctx: &mut Ctx,
    guest_allocation_ptr: RemotePtr,
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

pub fn from_guest_ptr<O: TryFrom<SerializedBytes>>(
    ctx: &mut Ctx,
    guest_allocation_ptr: RemotePtr,
) -> Result<O, WasmError>
where
    O::Error: Into<String>,
{
    let serialized_bytes: SerializedBytes =
        serialized_bytes_from_guest_ptr(ctx, guest_allocation_ptr)?;
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
    let host_allocation_ptr: AllocationPtr = payload.into();

    // this requires that the guest exported function being called knows what to do with a
    // host allocation pointer
    let guest_allocation_ptr: RemotePtr = match instance
        .call(
            call,
            &[Value::I64(host_allocation_ptr.as_remote_ptr().try_into()?)],
        )
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    let return_value: SerializedBytes = crate::guest::serialized_bytes_from_guest_ptr(
        instance.context_mut(),
        guest_allocation_ptr,
        // this ? might be a bit controversial as it means we return with an error WITHOUT telling the
        // guest that it can deallocate the return value
        // PROS:
        // - it's possible that we actually can't safely deallocate the return value here
        // - leaving the data in the guest may aid in debugging
        // - we avoid 'panicked while panicking' type situations
        // - slightly simpler code and clearer error handling
        // CONS:
        // - leaves 'memory leak' style cruft in the wasm guest
        //   (NOTE: all WASM memory is dropped when the instance is dropped anyway)
    )?;

    instance
        .call(
            "__deallocate_return_value",
            &[Value::I64(guest_allocation_ptr.try_into()?)],
        )
        .expect("deallocate return value error");

    Ok(return_value)
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
