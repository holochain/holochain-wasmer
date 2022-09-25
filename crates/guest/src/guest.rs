pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

use crate::allocation::consume_bytes;
use crate::allocation::write_bytes;

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident ),* ) => {
        extern "C" {
            $( pub fn $func_name(guest_allocation_ptr: $crate::GuestPtr, len: $crate::Len) -> $crate::GuestPtrLen; )*
        }
    };
}

/// Receive arguments from the host.
/// The guest sets the type `O` that the host needs to match.
/// If deserialization fails then a `GuestPtr` to a `WasmError::Deserialize` is returned.
/// The guest should __immediately__ return an `Err` back to the host.
/// The `WasmError::Deserialize` enum contains the bytes that failed to deserialize so the host can
/// unambiguously provide debug information.
#[inline(always)]
pub fn host_args<O>(ptr: GuestPtr, len: Len) -> Result<O, GuestPtrLen>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = consume_bytes(ptr, len);
    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(return_err_ptr(wasm_error!(WasmErrorInner::Deserialize(
                bytes.into()
            ))))
        }
    }
}

/// Given an extern that we expect the host to provide:
/// - Serialize the payload by reference
/// - Write the bytes into a new allocation on the guest side
/// - Call the host function and pass it the pointer and length to our leaked serialized data
/// - The host will consume and deallocate the bytes
/// - Deserialize whatever bytes we can import from the host after calling the host function
/// - Return a `Result` of the deserialized output type `O`
#[inline(always)]
pub fn host_call<I, O>(
    f: unsafe extern "C" fn(GuestPtr, Len) -> GuestPtrLen,
    input: I,
) -> Result<O, crate::WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Call the host function and receive the length of the serialized result.
    let input_bytes = holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e))?;
    let input_len: Len = match input_bytes.len().try_into() {
        Ok(len) => len,
        Err(e) => return Err(wasm_error!(e)),
    };
    let input_guest_ptr = crate::allocation::write_bytes(input_bytes);

    let (output_guest_ptr, output_len): (GuestPtr, Len) = split_u64(unsafe {
        // This is unsafe because all host function calls in wasm are unsafe.
        // The host will call `__deallocate` for us to free the leaked bytes from the input.
        f(input_guest_ptr, input_len)
    });

    // Deserialize the host bytes into the output type.
    let bytes = crate::allocation::consume_bytes(output_guest_ptr, output_len);
    match holochain_serialized_bytes::decode::<[u8], Result<O, WasmError>>(&bytes) {
        Ok(output) => Ok(output?),
        Err(e) => {
            tracing::error!(output_type = std::any::type_name::<O>(), ?bytes, "{}", e);
            Err(wasm_error!(WasmErrorInner::Deserialize(bytes.into())))
        }
    }
}

/// Convert any serializable value into a `GuestPtr` that can be returned to the host.
/// The host is expected to know how to consume and deserialize it.
#[inline(always)]
pub fn return_ptr<R>(return_value: R) -> GuestPtrLen
where
    R: Serialize + std::fmt::Debug,
{
    match holochain_serialized_bytes::encode::<Result<R, WasmError>>(&Ok(return_value)) {
        Ok(bytes) => {
            let len: Len = match bytes.len().try_into() {
                Ok(len) => len,
                Err(e) => return return_err_ptr(wasm_error!(e)),
            };
            merge_u64(write_bytes(bytes), len)
        }
        Err(e) => return_err_ptr(wasm_error!(WasmErrorInner::Serialize(e))),
    }
}

/// Convert a `WasmError` to a `GuestPtrLen` as best we can. This is not
/// necessarily straightforward as the serialization process can error recursively.
/// In the worst case we can't even serialize an enum variant, in which case we panic.
/// The casts from `usize` to `u32` are safe as long as the guest code is compiled
/// for `wasm32-unknown-unknown` target.
#[inline(always)]
pub fn return_err_ptr(wasm_error: WasmError) -> GuestPtrLen {
    match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(wasm_error)) {
        Ok(bytes) => {
            let len: Len = match bytes.len().try_into() {
                Ok(len) => len,
                Err(e) => return return_err_ptr(wasm_error!(e)),
            };
            merge_u64(write_bytes(bytes), len)
        }
        Err(e) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
            wasm_error!(WasmErrorInner::Serialize(e)),
        )) {
            Ok(bytes) => {
                let len: Len = match bytes.len().try_into() {
                    Ok(len) => len,
                    Err(e) => return return_err_ptr(wasm_error!(e)),
                };
                merge_u64(write_bytes(bytes), len)
            }
            // At this point we've errored while erroring
            Err(_) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                wasm_error!(WasmErrorInner::ErrorWhileError),
            )) {
                Ok(bytes) => {
                    let len: Len = match bytes.len().try_into() {
                        Ok(len) => len,
                        Err(e) => return return_err_ptr(wasm_error!(e)),
                    };
                    merge_u64(write_bytes(bytes), len)
                }
                // At this point we failed to serialize a unit varaint so IDK ¯\_(ツ)_/¯
                Err(_) => panic!("Failed to error"),
            },
        },
    }
}

/// A simple macro to wrap `return_err_ptr` in an analogy to the native rust `?`.
#[macro_export]
macro_rules! try_ptr {
    ( $e:expr, $fail:expr ) => {{
        match $e {
            Ok(v) => v,
            Err(e) => return return_err_ptr(wasm_error!("{}: {:?}", $fail, e)),
        }
    }};
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn wasm_error_macro_guest() {
        assert_eq!(
            wasm_error!("foo").error,
            WasmErrorInner::Guest("foo".into()),
        );

        assert_eq!(
            wasm_error!("{} {}", "foo", "bar").error,
            WasmErrorInner::Guest("foo bar".into())
        );

        assert_eq!(
            wasm_error!(WasmErrorInner::Host("foo".into())).error,
            WasmErrorInner::Host("foo".into()),
        );
    }
}
