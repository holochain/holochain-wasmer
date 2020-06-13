use crate::prelude::*;
use holochain_serialized_bytes::prelude::*;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

/// write a slice of bytes to the guest in a safe-ish way
///
/// a naive approach would look like this:
///
/// ```ignore
/// let view: MemoryView<u8> = ctx.memory(0).view();
/// unsafe {
///       std::ptr::copy_nonoverlapping(
///         slice.as_ptr(),
///         view.as_ptr().add(guest_ptr as usize) as *mut u8,
///         slice.len(),
///     );
/// }
/// ```
///
/// the guest memory is part of the host memory, so we get the host's pointer to the start of the
/// guest's memory with view.as_ptr() then we add the guest's pointer to where it wants to see the
/// written bytes then copy the slice directly across.
///
/// the problem with this approach is that the guest_ptr typically needs to be provided by the
/// allocator in the guest wasm in order to be safe for the guest's consumption, but a malicious
/// guest could provide bogus guest_ptr values that point outside the bounds of the guest memory.
/// the naive host would then corrupt its own memory by copying bytes... wherever, basically.
///
/// a better approach is to use wasmer's WasmPtr abstraction, which checks against the memory
/// bounds of the guest based on the input type and can be dereferenced to a [Cell] slice that we
/// can write to more safely.
///
/// @see https://docs.rs/wasmer-runtime-core/0.17.0/src/wasmer_runtime_core/memory/ptr.rs.html#120
///
/// this is still not completely safe in the face of shared memory and threads, etc.
pub fn write_bytes(ctx: &mut Ctx, guest_ptr: GuestPtr, slice: &[u8]) {
    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr as _);
    for (byte, cell) in slice.iter().zip(unsafe {
        ptr.deref_mut(ctx.memory(0), 0, slice.len() as _)
            .expect("pointer in bounds")
            .iter()
    }) {
        cell.set(*byte)
    }
}

/// read a slice of bytes from the guest in a safe-ish way
///
/// a naive approach would look like this:
///
/// ```ignore
/// let view: MemoryView<u8> = ctx.memory(0).view();
/// unsafe {
///     std::slice::from_raw_parts::<u8>(
///         view.as_ptr().add(guest_ptr as usize) as _,
///         len as _
///     )
/// }.to_vec()
/// ```
///
/// this is similar to the naive write_slice approach and has similar problems
/// @see write_slice()
///
/// a better approach is to use an immutable deref from a WasmPtr, which checks against memory
/// bounds for the guest, and map over the whole thing to a Vec<u8>
pub fn read_bytes(ctx: &Ctx, guest_ptr: GuestPtr, len: Len) -> Vec<u8> {
    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr as _);
    ptr.deref(ctx.memory(0), 0, len as _)
        .expect("pointer in bounds")
        .iter()
        .map(|cell| cell.get())
        .collect::<Vec<u8>>()
}

/// read an WasmSlice out of the guest from a guest pointer
pub fn read_wasm_slice(ctx: &Ctx, guest_ptr: GuestPtr) -> Result<slice::WasmSlice, WasmError> {
    Ok(read_bytes(ctx, guest_ptr, slice::WASM_SLICE_BYTES as Len).try_into()?)
}

/// read serialized bytes out of the guest from a guest pointer
pub fn read_serialized_bytes(
    ctx: &mut Ctx,
    guest_ptr: GuestPtr,
) -> Result<SerializedBytes, WasmError> {
    let slice = read_wasm_slice(ctx, guest_ptr)?;
    Ok(SerializedBytes::from(UnsafeBytes::from(
        read_bytes(ctx, slice.ptr(), slice.len()).to_vec(),
    )))
}

/// deserialize any SerializeBytes type out of the guest from a guest pointer
pub fn from_guest_ptr<O: TryFrom<SerializedBytes>>(
    ctx: &mut Ctx,
    guest_ptr: GuestPtr,
) -> Result<O, WasmError>
where
    O::Error: Into<String>,
{
    let serialized_bytes: SerializedBytes = read_serialized_bytes(ctx, guest_ptr)?;
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
    // get a pre-allocated guest pointer to write the input into
    let guest_input_ptr: GuestPtr = match instance
        .call(
            "__allocation_ptr_uninitialized",
            &[Value::I32(payload.bytes().len().try_into()?)],
        )
        .map_err(|e| WasmError::CallError(format!("{:?}", e)))?[0]
    {
        Value::I32(i) => i as GuestPtr,
        _ => unreachable!(),
    };

    let slice = read_wasm_slice(instance.context(), guest_input_ptr)?;

    // write the input payload into the guest at the offset specified by the allocation
    write_bytes(instance.context_mut(), slice.ptr(), payload.bytes());

    // call the guest function with its own pointer to its input
    // collect the guest's pointer to its output
    let guest_return_ptr: GuestPtr = match instance
        .call(call, &[Value::I32(guest_input_ptr.try_into()?)])
        .map_err(|e| WasmError::CallError(format!("{:?}", e)))?[0]
    {
        Value::I32(i) => i as GuestPtr,
        _ => unreachable!(),
    };

    let return_value: SerializedBytes = crate::guest::read_serialized_bytes(
        instance.context_mut(),
        guest_return_ptr,
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

    // tell the guest we are finished with the return pointer's data
    instance
        .call(
            "__deallocate_guest_allocation",
            &[Value::I32(guest_return_ptr.try_into()?)],
        )
        .map_err(|e| WasmError::CallError(format!("{:?}", e)))?;

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
