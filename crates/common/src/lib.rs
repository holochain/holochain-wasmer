pub mod allocation;
pub mod bytes;
pub mod result;
pub mod serialized_bytes;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;

pub type Ptr = u64;
pub type Len = u64;
pub type AllocationPtr = Ptr;
