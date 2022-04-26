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
    pub fn move_data_to_guest<I>(
        &self,
        input: I,
    ) -> Result<GuestPtrLen, wasmer_engine::RuntimeError>
    where
        I: serde::Serialize + std::fmt::Debug,
    {
        let data = holochain_serialized_bytes::encode(&input).map_err(|e| wasm_error!(e.into()))?;
        let guest_ptr: GuestPtr = match self
            .allocate_ref()
            .ok_or(wasm_error!(WasmErrorInner::Memory))?
            .call(&[Value::I32(
                data.len()
                    .try_into()
                    .map_err(|_| wasm_error!(WasmErrorInner::PointerMap))?,
            )])
            .map_err(|e| wasm_error!(WasmErrorInner::Host(e.to_string())))?[0]
        {
            Value::I32(guest_ptr) => guest_ptr as GuestPtr,
            _ => return Err(wasm_error!(WasmErrorInner::PointerMap).into()),
        };
        let len = data.len() as Len;
        crate::guest::write_bytes(
            self.memory_ref()
                .ok_or(wasm_error!(WasmErrorInner::Memory))?,
            guest_ptr,
            &data,
        )?;
        Ok(merge_u64(guest_ptr, len))
    }

    pub fn consume_bytes_from_guest<O>(
        &self,
        guest_ptr: GuestPtr,
        len: Len,
    ) -> Result<O, wasmer_engine::RuntimeError>
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
            .map_err(|e| wasm_error!(WasmErrorInner::Host(e.to_string())))?;
        match holochain_serialized_bytes::decode(&bytes) {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
                Err(wasm_error!(e.into()).into())
            }
        }
    }
}
