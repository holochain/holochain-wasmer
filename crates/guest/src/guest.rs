pub mod allocation;

pub extern crate holochain_serialized_bytes;
pub use holochain_wasmer_common::*;

use crate::allocation::consume_bytes;
use crate::allocation::write_bytes;

pub use paste::paste;

#[macro_export]
macro_rules! host_externs {
    ( $( $func_name:ident:$version:literal ),* ) => {
        $crate::paste! {
            #[no_mangle]
            extern "C" {
                $( pub fn [<__hc__ $func_name _ $version>](guest_allocation_ptr: usize, len: usize) -> $crate::DoubleUSize; )*
            }
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
pub fn host_args<O>(ptr: usize, len: usize) -> Result<O, DoubleUSize>
where
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    let bytes = consume_bytes(ptr, len);
    match holochain_serialized_bytes::decode(&bytes) {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
            Err(return_err_ptr(wasm_error!(WasmErrorInner::Deserialize(
                bytes
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
    f: unsafe extern "C" fn(usize, usize) -> DoubleUSize,
    input: I,
) -> Result<O, crate::WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Call the host function and receive the length of the serialized result.
    let mut input_bytes = holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e))?;
    input_bytes.shrink_to_fit();
    if input_bytes.capacity() != input_bytes.len() {
        tracing::warn!("Capacity should equal length, dealloc will fail");
    }
    debug_assert!(
        input_bytes.capacity() == input_bytes.len(),
        "Capacity should equal length, dealloc would fail"
    );
    let input_len: usize = input_bytes.len();
    let input_guest_ptr = crate::allocation::write_bytes(input_bytes);

    let (output_guest_ptr, output_len): (usize, usize) = split_usize(unsafe {
        // This is unsafe because all host function calls in wasm are unsafe.
        // The host will call `__hc__deallocate_1` for us to free the leaked bytes from the input.
        f(input_guest_ptr, input_len)
    })?;

    // Deserialize the host bytes into the output type.
    let bytes = crate::allocation::consume_bytes(output_guest_ptr, output_len);
    match holochain_serialized_bytes::decode::<[u8], Result<O, WasmError>>(&bytes) {
        Ok(output) => Ok(output?),
        Err(e) => {
            tracing::error!(output_type = std::any::type_name::<O>(), ?bytes, "{}", e);
            Err(wasm_error!(WasmErrorInner::Deserialize(bytes)))
        }
    }
}

/// Convert any serializable value into a `GuestPtr` that can be returned to the host.
/// The host is expected to know how to consume and deserialize it.
#[inline(always)]
pub fn return_ptr<R>(return_value: R) -> DoubleUSize
where
    R: Serialize + std::fmt::Debug,
{
    match holochain_serialized_bytes::encode::<Result<R, WasmError>>(&Ok(return_value)) {
        Ok(mut bytes) => {
            let len: usize = bytes.len();
            bytes.shrink_to_fit();
            if bytes.capacity() != bytes.len() {
                tracing::warn!("Capacity should equal length, dealloc will fail");
            }
            debug_assert!(
                bytes.capacity() == bytes.len(),
                "Capacity should equal length, dealloc would fail"
            );
            merge_usize(write_bytes(bytes), len).unwrap_or_else(return_err_ptr)
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
pub fn return_err_ptr(wasm_error: WasmError) -> DoubleUSize {
    let mut bytes =
        match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(wasm_error)) {
            Ok(bytes) => bytes,
            Err(e) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                wasm_error!(WasmErrorInner::Serialize(e)),
            )) {
                Ok(bytes) => bytes,
                // At this point we've errored while erroring
                Err(_) => match holochain_serialized_bytes::encode::<Result<(), WasmError>>(&Err(
                    wasm_error!(WasmErrorInner::ErrorWhileError),
                )) {
                    Ok(bytes) => bytes,
                    // At this point we failed to serialize a unit variant so IDK ¯\_(ツ)_/¯
                    Err(_) => panic!("Failed to error"),
                },
            },
        };
    bytes.shrink_to_fit();
    if bytes.capacity() != bytes.len() {
        tracing::warn!("Capacity should equal length, dealloc will fail");
    }
    debug_assert!(
        bytes.capacity() == bytes.len(),
        "Capacity should equal length, dealloc would fail"
    );
    let len = bytes.len();
    merge_usize(write_bytes(bytes), len).expect("Failed to build return value")
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
