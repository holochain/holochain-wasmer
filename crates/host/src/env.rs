use std::num::TryFromIntError;

use crate::guest::read_bytes;
use crate::prelude::*;
use wasmer::Function;
use wasmer::LazyInit;
use wasmer::Memory;
use wasmer::WasmerEnv;

#[derive(Clone, Default, WasmerEnv)]
pub struct Env {
    #[wasmer(export)]
    memory: LazyInit<Memory>,
    #[wasmer(export(name = "__allocate"))]
    allocate: LazyInit<Function>,
    #[wasmer(export(name = "__deallocate"))]
    deallocate: LazyInit<Function>,
}

impl Env {
    /// Given some input I that can be serialized, request an allocation from the
    /// guest and copy the serialized bytes to the allocated pointer. The guest
    /// MUST subsequently take ownership of these bytes or it will leak memory.
    pub fn move_data_to_guest<I>(&self, input: I) -> Result<GuestPtrLen, wasmer::RuntimeError>
    where
        I: serde::Serialize + std::fmt::Debug,
    {
        let data = holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e))?;
        let guest_ptr: GuestPtr = match self
            .allocate_ref()
            .ok_or(wasm_error!(WasmErrorInner::Memory))?
            .call(&[Value::I32(
                data.len()
                    .try_into()
                    .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
            )])
            .map_err(|e| wasm_error!(e.to_string()))?
            .get(0)
        {
            Some(Value::I32(guest_ptr)) => (*guest_ptr)
                .try_into()
                .map_err(|e: TryFromIntError| wasm_error!(e))?,
            _ => return Err(wasm_error!(WasmErrorInner::PointerMap).into()),
        };
        let len: Len = match data.len().try_into() {
            Ok(len) => len,
            Err(e) => return Err(wasm_error!(e).into()),
        };
        crate::guest::write_bytes(
            self.memory_ref()
                .ok_or(wasm_error!(WasmErrorInner::Memory))?,
            guest_ptr,
            &data,
        )?;
        Ok(merge_u64(guest_ptr, len))
    }

    /// Given a pointer and length for a region of memory in the guest, copy the
    /// bytes to the host and attempt to deserialize type `O` from the data. The
    /// guest will be asked to deallocate the copied bytes whether or not the
    /// deserialization is successful.
    pub fn consume_bytes_from_guest<O>(
        &self,
        guest_ptr: GuestPtr,
        len: Len,
    ) -> Result<O, wasmer::RuntimeError>
    where
        O: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let bytes = read_bytes(
            self.memory_ref()
                .ok_or(wasm_error!(WasmErrorInner::Memory))?,
            guest_ptr,
            len,
        )?;
        self.deallocate_ref()
            .ok_or(wasm_error!(WasmErrorInner::Memory))?
            .call(&[
                Value::I32(
                    guest_ptr
                        .try_into()
                        .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
                ),
                Value::I32(
                    len.try_into()
                        .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
                ),
            ])
            .map_err(|e| wasm_error!(e.to_string()))?;
        match holochain_serialized_bytes::decode(&bytes) {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
                Err(wasm_error!(e).into())
            }
        }
    }
}
