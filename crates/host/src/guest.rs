use crate::prelude::*;
use core::num::TryFromIntError;
use holochain_serialized_bytes::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::Value;

/// Write a slice of bytes to the guest in a safe-ish way.
///
/// A naive approach would look like this:
///
/// ```ignore
/// let view: MemoryView<u8> = ctx.memory(0).view();
/// unsafe {
///       std::ptr::copy_nonoverlapping(
///         slice.as_ptr(),
///         view.as_ptr().add(guest_ptr) as *mut u8,
///         slice.len(),
///     );
/// }
/// ```
///
/// The guest memory is part of the host memory, so we get the host's pointer to the start of the
/// guest's memory with `view.as_ptr()`, then we add the guest's pointer to where it wants to see the
/// written bytes, then copy the slice directly across.
///
/// The problem with this approach is that the `guest_ptr` typically needs to be provided by the
/// allocator in the guest wasm in order to be safe for the guest's consumption, but a malicious
/// guest could provide bogus `guest_ptr` values that point outside the bounds of the guest memory.
/// The naive host would then corrupt its own memory by copying bytes... wherever, basically.
///
/// A better approach is to use wasmer's `WasmPtr` abstraction, which checks against the memory
/// bounds of the guest based on the input type and can be dereferenced to a [Cell] slice that we
/// can write to more safely.
///
/// @see https://docs.rs/wasmer-runtime-core/0.17.0/src/wasmer_runtime_core/memory/ptr.rs.html#120
///
/// This is still not completely safe in the face of shared memory and threads, etc.
///
/// The guest needs to provide a pointer to a pre-allocated (e.g. by leaking a Vec<u8>) region
/// of the guest's memory that is safe for the host to write to.
///
/// It is the host's responsibility to tell the guest the length of the allocation that is needed
/// and the guest's responsibility to correctly reserve an allocation to be written into.
///
/// `write_bytes()` takes a slice of bytes and writes it to the position at the guest pointer.
///
/// The guest and the host negotiate the length of the bytes separately.
///
/// @see read_bytes()
pub fn write_bytes(
    memory: &Memory,
    guest_ptr: GuestPtr,
    slice: &[u8],
) -> Result<(), wasmer_engine::RuntimeError> {
    let len: Len = match slice.len().try_into() {
        Ok(len) => len,
        Err(e) => return Err(wasm_error!(e).into()),
    };
    #[cfg(feature = "debug_memory")]
    tracing::debug!("writing bytes from host to guest at: {} {}", guest_ptr, len);

    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr);
    // Write the length prefix immediately before the slice at the guest pointer position.
    for (byte, cell) in slice.iter().zip(
        ptr.deref(memory, 0, len)
            .ok_or(wasm_error!(WasmErrorInner::Memory))?
            .iter(),
    ) {
        cell.set(*byte)
    }

    Ok(())
}

/// Read a slice of bytes from the guest in a safe-ish way.
///
/// A naive approach would look like this:
///
/// ```ignore
/// let view: MemoryView<u8> = ctx.memory(0).view();
/// unsafe {
///     std::slice::from_raw_parts::<u8>(
///         view.as_ptr().add(guest_ptr),
///         len
///     )
/// }.to_vec()
/// ```
///
/// This is similar to the naive write_slice approach and has similar problems.
/// @see write_slice()
///
/// A better approach is to use an immutable deref from a `WasmPtr`, which checks against memory
/// bounds for the guest, and map over the whole thing to a `Vec<u8>`.
pub fn read_bytes(
    memory: &Memory,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<Vec<u8>, wasmer_engine::RuntimeError> {
    #[cfg(feature = "debug_memory")]
    tracing::debug!("reading bytes from guest to host at: {} {}", guest_ptr, len);

    let ptr: WasmPtr<u8, Array> = WasmPtr::new(guest_ptr);
    Ok(ptr
        .deref(memory, 0, len)
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
            Err(wasm_error!(e).into())
        }
    }
}

/// Host calling guest for the function named `call` with the given `payload` in a vector of bytes
/// result is either a vector of bytes from the guest found at the location of the returned guest
/// allocation pointer or a `RuntimeError` built from a `WasmError`.
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
        holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e))?;

    // Get a pre-allocated guest pointer to write the input into.
    let guest_input_length = payload
        .len()
        .try_into()
        .map_err(|e: TryFromIntError| wasm_error!(WasmErrorInner::CallError(e.to_string())))?;
    let guest_input_length_value: Value = Value::I32(guest_input_length);
    let (guest_input_ptr, guest_input_ptr_value) = match instance
        .exports
        .get_function("__allocate")
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(&[guest_input_length_value.clone()])
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .get(0)
    {
        Some(Value::I32(guest_input_ptr)) => (
            (*guest_input_ptr)
                .try_into()
                .map_err(|e: TryFromIntError| {
                    wasm_error!(WasmErrorInner::CallError(e.to_string()))
                })?,
            Value::I32(*guest_input_ptr),
        ),
        _ => {
            return Err(wasm_error!(WasmErrorInner::CallError(
                "Not I32 return from __allocate".to_string()
            ))
            .into())
        }
    };

    // Write the input payload into the guest at the offset specified by the allocation.
    write_bytes(
        instance
            .exports
            .get_memory("memory")
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
        .call(&[guest_input_ptr_value, guest_input_length_value])
    {
        Ok(v) => match v.get(0) {
            Some(Value::I64(i)) => {
                let u: GuestPtrLen = (*i)
                    .try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e))?;
                split_u64(u)
            }
            _ => return Err(wasm_error!(WasmErrorInner::PointerMap).into()),
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
                            Err(wasm_error!(e).into())
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
        instance
            .exports
            .get_memory("memory")
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
                    .map_err(|e: TryFromIntError| wasm_error!(e))?,
            ),
            Value::I32(
                len.try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e))?,
            ),
        ])
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(format!("{:?}", e))))?;

    return_value.map_err(|e| e.into())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn wasm_error_macro_host() {
        assert_eq!(wasm_error!("foo").error, WasmErrorInner::Host("foo".into()),);

        assert_eq!(
            wasm_error!("{} {}", "foo", "bar").error,
            WasmErrorInner::Host("foo bar".into())
        );

        assert_eq!(
            wasm_error!(WasmErrorInner::Host("foo".into())).error,
            WasmErrorInner::Host("foo".into()),
        );
    }
}
