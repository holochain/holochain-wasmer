use crate::guest;
use crate::prelude::*;
use wasmer_runtime::Ctx;

pub fn __import_data(ctx: &mut Ctx, guest_ptr: GuestPtr) {
    if !ctx.data.is_null() {
        let b: Box<SerializedBytes> = unsafe { Box::from_raw(ctx.data as _) };
        let wasm_fat_ptr = guest::read_wasm_slice(ctx, guest_ptr).unwrap();
        guest::write_bytes(ctx, wasm_fat_ptr.ptr(), &*b.bytes());
    }
    ctx.data = std::ptr::null::<SerializedBytes>() as _;
}

/// always call this before setting and after using a context
/// it guards against badly behaved host/guest logic by freeing any previously leaked data pointed
/// at by the context data
#[allow(unused_assignments)]
pub fn unleak_context_data(data: *mut std::ffi::c_void) {
    if !data.is_null() {
        // unleak the old contents on the assumption that it is SerializedBytes
        // this assumption basically assumes that the only thing setting context data is the
        // set_context_data function below
        let _: Box<SerializedBytes> = unsafe { Box::from_raw(data as _) };
    }
}

pub fn set_context_data(ctx: &mut Ctx, serialized_bytes: SerializedBytes) -> Len {
    unleak_context_data(ctx.data);

    // leak the provided serialized bytes into the context data so it can be imported later
    let len = serialized_bytes.bytes().len();
    let b = Box::new(serialized_bytes);
    ctx.data = Box::into_raw(b) as _;
    len as Len
}
