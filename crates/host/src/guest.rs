use crate::prelude::*;
use core::num::TryFromIntError;
use holochain_serialized_bytes::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::Value;

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
///
/// the guest needs to provide a pointer to a pre-allocated (e.g. by forgetting a Vec<u8>) region
/// of the guest's memory that it is safe for the host to write to.
///
/// it is the host's responsibility to tell the guest the length of the allocation that is needed
/// and the guest's responsibility to correctly reserve an allocation to be written into.
///
/// write_bytes() takes a slice of bytes and writes it to the position at the guest pointer
///
/// as the byte slice cannot be co-ordinated by the compiler (because the host and guest have
/// different compilers and allocators) we prefix the allocation with a WasmSize length value.
///
/// for example, if we wanted to write the slice &[1, 2, 3] then we'd take the length of the slice,
/// 3 as a WasmSize, which is u32, i.e. a 3_u32 and convert it to an array of u8 bytes as
/// [ 3_u8, 0_u8, 0_u8, 0_u8 ] and concatenate it to our original [ 1_u8, 2_u8, 3_u8 ].
/// this gives the full array of bytes to write as:
///
/// ```ignore
/// [ 3_u8, 0_u8, 0_u8, 0_u8, 1_u8, 2_u8, 3_u8 ]
/// ```
///
/// this allows us to read back the byte slice given only a GuestPtr because the read operation
/// can do the inverse in a single step by reading the length inline
///
/// it also requires the host and the guest to both adopt this convention and read/write the
/// additional 4 byte prefix in order to read/write the real payload correctly
///
/// @see read_bytes()
pub fn write_bytes(
    memory: &Memory,
    guest_ptr: GuestPtr,
    slice: &[u8],
) -> Result<(), wasmer_engine::RuntimeError> {
    #[cfg(feature = "debug_memory")]
    tracing::debug!(
        "writing bytes from host to guest at: {} {}",
        guest_ptr as u32,
        slice.len() as u32
    );

    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr as _);
    // write the length prefix immediately before the slice at the guest pointer position
    for (byte, cell) in slice.iter().zip(
        ptr.deref(memory, 0 as GuestPtr, slice.len() as Len)
            .ok_or(wasm_error!(WasmErrorInner::Memory))?
            .iter(),
    ) {
        cell.set(*byte)
    }

    Ok(())
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
///
/// this does the inverse of write_bytes to read a vector of arbitrary length given only a single
/// GuestPtr value
///
/// it reads the first 4 u8 bytes at the GuestPtr position and interprets them as a single u32
/// value representing a Len which is the length of the return Vec<u8> to read at position
/// GuestPtr + 4
///
/// using the example in write_bytes(), if we had written
///
/// ```ignore
/// [ 3_u8, 0_u8, 0_u8, 0_u8, 1_u8, 2_u8, 3_u8 ]
/// ```
///
/// and this returned a GuestPtr to `5678` then we would read it back by taking the first 4 bytes
/// at `5678` which would be `[ 3_u8, 0_u8, 0_u8, 0_u8 ]` which we interpret as the length `3_u32`.
///
/// we then read the length 3 bytes from position `5682` (ptr + 4) to get our originally written
/// bytes of `[ 1_u8, 2_u8, 3_u8 ]`.
pub fn read_bytes(
    memory: &Memory,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<Vec<u8>, wasmer_engine::RuntimeError> {
    #[cfg(feature = "debug_memory")]
    tracing::debug!(
        "reading bytes from guest to host at: {} {}",
        guest_ptr as u32,
        len as u32
    );

    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr as _);
    Ok(ptr
        .deref(memory, 0, len as _)
        .ok_or(wasm_error!(WasmErrorInner::Memory))?
        .iter()
        .map(|cell| cell.get())
        .collect::<Vec<u8>>())
}

/// Deserialize any DeserializeOwned type out of the guest from a guest pointer.
pub fn from_guest_ptr<O>(
    memory: &Memory,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<O, wasmer_engine::RuntimeError>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = read_bytes(memory, guest_ptr, len)?;
    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(wasm_error!(e.into()).into())
        }
    }
}

/// Host calling guest for the function named `call` with the given `payload` in a vector of bytes
/// result is either a vector of bytes from the guest found at the location of the returned guest
/// allocation pointer or a `WasmError`.
pub fn call<I, O>(
    instance: Arc<Mutex<Instance>>,
    f: &str,
    input: I,
) -> Result<O, wasmer_engine::RuntimeError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let instance = instance.lock();
    // The guest will use the same crate for decoding if it uses the wasm common crate.
    let payload: Vec<u8> =
        holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e.into()))?;

    // Get a pre-allocated guest pointer to write the input into.
    let guest_input_ptr: GuestPtr = match instance
        .exports
        .get_function("__allocate")
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(&[Value::I32(
            payload
                .len()
                .try_into()
                .map_err(|e: TryFromIntError| wasm_error!(e.into()))?,
        )])
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?[0]
    {
        Value::I32(i) => i as GuestPtr,
        _ => unreachable!(),
    };

    // Write the input payload into the guest at the offset specified by the allocation.
    write_bytes(
        &instance
            .exports
            // potentially snake oil
            // https://github.com/wasmerio/wasmer/issues/2780#issuecomment-1054452629
            .get_with_generics_weak("memory")
            .map_err(|_| wasm_error!(WasmErrorInner::Memory))?,
        guest_input_ptr,
        &payload,
    )?;

    // Call the guest function with its own pointer to its input.
    // Collect the guest's pointer to its output.
    let (guest_return_ptr, len): (GuestPtr, Len) = match instance
        .exports
        .get_function(f)
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(&[
            Value::I32(
                guest_input_ptr
                    .try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e.into()))?,
            ),
            Value::I32(
                payload
                    .len()
                    .try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e.into()))?,
            ),
        ]) {
        Ok(v) => match v[0] {
            Value::I64(i) => split_u64(i as _),
            _ => unreachable!(),
        },
        Err(e) => match e.downcast::<WasmError>() {
            Ok(WasmError { file, line, error }) => match error {
                WasmErrorInner::HostShortCircuit(encoded) => {
                    return match holochain_serialized_bytes::decode(&encoded) {
                        Ok(v) => Ok(v),
                        Err(e) => {
                            tracing::error!(
                                input_type = std::any::type_name::<O>(),
                                ?encoded,
                                "{}",
                                e
                            );
                            Err(wasm_error!(e.into()).into())
                        }
                    }
                }
                _ => return Err(WasmError { file, line, error }.into()),
            },
            Err(e) => return Err(wasm_error!(WasmErrorInner::CallError(e.to_string())).into()),
        },
    };

    // We ? here to return early WITHOUT calling deallocate.
    // The host MUST discard any wasm instance that errors at this point to avoid memory leaks.
    // The WasmError in the result type here is for deserializing out of the guest.
    let return_value: Result<O, WasmError> = from_guest_ptr(
        &instance
            .exports
            // maybe snake oil but:
            // https://github.com/wasmerio/wasmer/issues/2780#issuecomment-1054452629
            .get_with_generics_weak("memory")
            .map_err(|_| wasm_error!(WasmErrorInner::Memory))?,
        guest_return_ptr,
        len,
    )?;

    // Tell the guest we are finished with the return pointer's data.
    instance
        .exports
        .get_function("__deallocate")
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(&[
            Value::I32(
                guest_return_ptr
                    .try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e.into()))?,
            ),
            Value::I32(
                len.try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e.into()))?,
            ),
        ])
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(format!("{:?}", e))))?;

    return_value.map_err(|e| e.into())
}
