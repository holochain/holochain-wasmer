use crate::env::Env;
use crate::prelude::*;

/// Simple wrapper around the env to move data from the env to the guest.
pub fn __import_data(env: &Env) -> Result<GuestPtrLen, wasmer_engine::RuntimeError> {
    env.move_data_to_guest()
}
