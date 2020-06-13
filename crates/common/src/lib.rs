pub mod fat_ptr;
pub mod result;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;

/// something like usize for wasm
/// wasm has a memory limit of 4GB so offsets and lengths fit in u32
///
/// the host needs to directly read and write to the guest's memory so we need a predictable number
/// of bytes to represent offsets and lengths
/// we don't want to have to recompile every wasm as u32 and u64 to match different `usize` sizes
/// on the host, especially considering that u64 offsets/lengths would add no value to wasm
/// it's much more important that the host can reliably work with the guest memory without breaking
/// the guest's allocator
///
/// wasmer itself uses u32 in the WasmPtr abstraction etc.
/// @see https://docs.rs/wasmer-runtime/0.17.0/wasmer_runtime/struct.WasmPtr.html
pub type WasmSize = u32;

pub type Len = WasmSize;
pub type GuestPtr = WasmSize;
