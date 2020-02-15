extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate holochain_json_derive;

pub mod allocation;
pub mod bytes;
pub mod error;
pub mod json;
pub mod result;
pub mod string;

pub use result::*;

pub type Ptr = u64;
pub type Len = u64;
pub type AllocationPtr = Ptr;
