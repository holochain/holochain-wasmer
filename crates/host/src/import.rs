use crate::guest;
use crate::prelude::*;
use wasmer_runtime::Ctx;

/// Dump the current `Ctx` data wherever the guest asks us to.
/// The `Ctx` data is cleared to a null ptr in the process.
pub fn __import_data(ctx: &mut Ctx, guest_ptr: GuestPtr) -> Result<(), WasmError> {
    if !ctx.data.is_null() {
        let b: Box<Vec<u8>> = unsafe { Box::from_raw(ctx.data as _) };
        guest::write_bytes(ctx, guest_ptr, &*b)?;
    }
    ctx.data = std::ptr::null::<Vec<u8>>() as _;
    Ok(())
}

/// Always call this before setting and after using a context.
/// It guards against badly behaved host/guest logic by freeing any previously leaked data pointed
/// at by the context data.
#[allow(unused_assignments)]
pub fn free_context_data(data: *mut std::ffi::c_void) {
    if !data.is_null() {
        // Unleak the old contents, assuming it is a `Vec<u8>`.
        // This assumption basically assumes that the only thing setting context data is the
        // `set_context_data` function below.
        let _: Box<Vec<u8>> = unsafe { Box::from_raw(data as _) };
    }
}

/// Set the `Ctx` data as a `Vec<u8>` for any serializable input.
pub fn set_context_data<I>(ctx: &mut Ctx, input: I) -> Result<Len, WasmError>
where
    I: serde::Serialize + std::fmt::Debug,
{
    // Guard against the situation where some bad code sets a new Ctx.data value while some other
    // data is leaked in memory, free it before setting a new value.
    free_context_data(ctx.data);

    // Leak the provided serialized bytes into the context data so it can be imported later.
    let data: Vec<u8> = holochain_serialized_bytes::encode(&input).map_err(|e| {
        WasmError::new(
            WasmErrorType::Serialize(e),
            "Host failed to serialize output for the guest",
        )
    })?;
    let len = data.len();
    let b = Box::new(data);
    ctx.data = Box::into_raw(b) as _;
    Ok(len as Len)
}
