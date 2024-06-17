use crate::prelude::*;
use core::num::TryFromIntError;
use holochain_serialized_bytes::prelude::*;
use std::sync::Arc;
use wasmer::Instance;
use wasmer::Memory;
use wasmer::MemoryView;
use wasmer::StoreMut;
use wasmer::Value;
use wasmer::WasmSlice;

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
    store_mut: &mut StoreMut,
    memory: &Memory,
    guest_ptr: GuestPtr,
    slice: &[u8],
) -> Result<(), wasmer::RuntimeError> {
    let len: Len = match slice.len().try_into() {
        Ok(len) => len,
        Err(e) => return Err(wasm_error!(e).into()),
    };
    #[cfg(feature = "debug_memory")]
    tracing::debug!("writing bytes from host to guest at: {} {}", guest_ptr, len);

    WasmSlice::new(&memory.view(store_mut), guest_ptr.into(), len.into())?.write_slice(slice)?;

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
    memory_view: &MemoryView,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<Vec<u8>, wasmer::MemoryAccessError> {
    #[cfg(feature = "debug_memory")]
    tracing::debug!("reading bytes from guest to host at: {} {}", guest_ptr, len);

    WasmSlice::new(memory_view, guest_ptr.into(), len.into())?.read_to_vec()
}

/// Deserialize any DeserializeOwned type out of the guest from a guest pointer.
pub fn from_guest_ptr<O>(
    store_mut: &mut StoreMut,
    memory: &Memory,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<O, wasmer::RuntimeError>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes =
        WasmSlice::new(&memory.view(store_mut), guest_ptr.into(), len.into())?.read_to_vec()?;

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
/// The reason that this takes a separate store and instance is that the host does not neccessarily
/// have access to an InstanceWithStore, such as the case when the guest is called from within a
/// host function call.
pub fn call<I, O>(
    store_mut: &mut StoreMut,
    instance: Arc<Instance>,
    f: &str,
    input: I,
) -> Result<O, wasmer::RuntimeError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
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
        .get_function("__hc__allocate_1")
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(store_mut, &[guest_input_length_value.clone()])
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .first()
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
                "Not I32 return from __hc__allocate_1".to_string()
            ))
            .into())
        }
    };

    // Write the input payload into the guest at the offset specified by the allocation.
    write_bytes(
        store_mut,
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
        .call(
            store_mut,
            &[guest_input_ptr_value, guest_input_length_value],
        ) {
        Ok(v) => match v.first() {
            Some(Value::I64(i)) => {
                let u: GuestPtrLen = (*i)
                    .try_into()
                    .map_err(|e: TryFromIntError| wasm_error!(e))?;
                split_u64(u).map_err(WasmHostError)?
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
                _ => return Err(WasmHostError(WasmError { file, line, error }).into()),
            },
            Err(e) => return Err(wasm_error!(WasmErrorInner::CallError(e.to_string())).into()),
        },
    };

    // We ? here to return early WITHOUT calling deallocate.
    // The host MUST discard any wasm instance that errors at this point to avoid memory leaks.
    // The WasmError in the result type here is for deserializing out of the guest.
    let return_value: Result<O, WasmError> = from_guest_ptr(
        store_mut,
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
        .get_function("__hc__deallocate_1")
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(e.to_string())))?
        .call(
            store_mut,
            &[
                Value::I32(
                    guest_return_ptr
                        .try_into()
                        .map_err(|e: TryFromIntError| wasm_error!(e))?,
                ),
                Value::I32(
                    len.try_into()
                        .map_err(|e: TryFromIntError| wasm_error!(e))?,
                ),
            ],
        )
        .map_err(|e| wasm_error!(WasmErrorInner::CallError(format!("{:?}", e))))?;

    return_value.map_err(|e| WasmHostError(e).into())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn wasm_error_macro_host() {
        assert_eq!(
            wasm_error!("foo").0.error,
            WasmErrorInner::Host("foo".into()),
        );

        assert_eq!(
            wasm_error!("{} {}", "foo", "bar").0.error,
            WasmErrorInner::Host("foo bar".into())
        );

        assert_eq!(
            wasm_error!(WasmErrorInner::Host("foo".into())).0.error,
            WasmErrorInner::Host("foo".into()),
        );
    }
}
