use crate::env::Env;
use crate::prelude::*;

/// Dump the current `Ctx` data wherever the guest asks us to.
/// The `Ctx` data is cleared to a null ptr in the process.
pub fn __import_data(env: &Env) -> Result<GuestPtr, WasmError> {
    env.write_host_return()
}
