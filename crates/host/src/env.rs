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
    pub(crate) host_return_encoded: Arc<RwLock<Vec<u8>>>,
}

impl Env {
    pub fn set_host_return_encoded<I>(&self, input: I) -> Result<Len, WasmError>
    where
        I: serde::Serialize + std::fmt::Debug,
    {
        *self.host_return_encoded.write() = holochain_serialized_bytes::encode(&input)?;
        Ok(self.host_return_encoded.read().len() as Len)
    }

    pub fn clear_host_return_encoded(&self) {
        *self.host_return_encoded.write() = Vec::new();
    }

    pub fn write_host_return(&self) -> Result<GuestPtr, WasmError> {
        let guest_ptr: GuestPtr = match self
            .allocate_ref()
            .ok_or(WasmError::Memory)?
            .call(&[Value::I32(
                self.host_return_encoded
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
            &self.host_return_encoded.read(),
        )?;
        self.clear_host_return_encoded();
        Ok(guest_ptr)
    }
}
