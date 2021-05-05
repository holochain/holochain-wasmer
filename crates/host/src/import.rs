use crate::env::Env;
use crate::prelude::*;

/// Dump the current `Ctx` data wherever the guest asks us to.
/// The `Ctx` data is cleared to a null ptr in the process.
pub fn __import_data(env: &Env, guest_ptr: GuestPtr) -> Result<(), WasmError> {
    guest::write_bytes(
        env.memory_ref().ok_or(WasmError::Memory)?,
        guest_ptr,
        &env.host_return_encoded.read(),
    )?;
    *env.host_return_encoded.write() = Vec::new();
    Ok(())
}

/// Set the `Env` data as a `Vec<u8>` for any serializable input.
pub fn set_host_return_encoded<I>(env: &Env, input: I) -> Result<Len, WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
{
    *env.host_return_encoded.write() = holochain_serialized_bytes::encode(&input)?;
    Ok(env.host_return_encoded.read().len() as Len)
}
