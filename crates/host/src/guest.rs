use crate::prelude::*;
use byte_slice_cast::AsSliceOf;
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
pub fn write_slice(ctx: &mut Ctx, guest_ptr: RemotePtr, slice: &[u8]) {
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
pub fn read_slice(ctx: &mut Ctx, guest_ptr: RemotePtr, len: Len) -> Vec<u8> {
    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr as _);
    ptr.deref(ctx.memory(0), 0, len as _)
        .expect("pointer in bounds")
        .iter()
        .map(|cell| cell.get())
        .collect::<Vec<u8>>()
}

pub fn serialized_bytes_from_guest_ptr(
    ctx: &mut Ctx,
    guest_allocation_ptr: RemotePtr,
) -> Result<SerializedBytes, WasmError> {
    let bytes_vec: Vec<u8> = read_slice(
        ctx,
        guest_allocation_ptr,
        allocation::ALLOCATION_BYTES_ITEMS as Len,
    );
    let guest_allocation: allocation::Allocation = bytes_vec.as_slice_of::<u64>()?.try_into()?;

    Ok(SerializedBytes::from(UnsafeBytes::from(
        read_slice(ctx, guest_allocation[0], guest_allocation[1]).to_vec(),
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
    // @TODO this is insecure because it leaks the payload and relies on the guest to consume it
    // with host_args!()
    // if the guest never consumes this ptr then the payload stays leaked so we want to fix/guard
    // against that by dropping the payload bytes if the guest never uses them
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
