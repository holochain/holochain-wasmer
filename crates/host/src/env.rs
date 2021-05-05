use crate::guest::read_bytes;
use crate::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
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
    data: Arc<RwLock<Vec<u8>>>,
}

impl Env {
    pub fn set_data<I>(&self, input: I) -> Result<(), WasmError>
    where
        I: serde::Serialize + std::fmt::Debug,
    {
        *self.data.write() = holochain_serialized_bytes::encode(&input)?;
        Ok(())
    }

    pub fn move_data_to_guest(&self) -> Result<GuestPtr, WasmError> {
        let guest_ptr: GuestPtr = match self
            .allocate_ref()
            .ok_or(WasmError::Memory)?
            .call(&[Value::I32(
                self.data
                    .read()
                    .len()
                    .try_into()
                    .map_err(|_| WasmError::PointerMap)?,
            )])
            .map_err(|e| WasmError::Host(e.to_string()))?[0]
        {
            Value::I32(guest_ptr) => guest_ptr as GuestPtr,
            _ => Err(WasmError::PointerMap)?,
        };
        crate::guest::write_bytes(
            self.memory_ref().ok_or(WasmError::Memory)?,
            guest_ptr,
            &self.data.read(),
        )?;
        *self.data.write() = Vec::new();
        Ok(guest_ptr)
    }

    pub fn consume_bytes_from_guest_ptr<O>(&self, guest_ptr: GuestPtr) -> Result<O, WasmError>
    where
        O: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let bytes = read_bytes(self.memory_ref().ok_or(WasmError::Memory)?, guest_ptr)?;
        self.deallocate_ref()
            .ok_or(WasmError::Memory)?
            .call(&[Value::I32(
                guest_ptr.try_into().map_err(|_| WasmError::PointerMap)?,
            )])
            .map_err(|e| WasmError::Host(e.to_string()))?;
        match holochain_serialized_bytes::decode(&bytes) {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(input_type = std::any::type_name::<O>(), bytes = ?bytes, "{}", e);
                Err(e.into())
            }
        }
    }
}
