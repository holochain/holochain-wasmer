pub mod allocation;
pub mod result;
pub mod serialized_bytes;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;

pub type Ptr = u64;
pub type Len = u64;

/// an unwrapped AllocationPtr that is on the other side of the host/wasm boundary
pub type RemotePtr = Ptr;

/// AllocationPtr wraps a ptr that is used to pass the location of an Allocation
/// between the host and guest (in either direction).
/// The AllocationPtr intentionally does not implement Clone
/// The From<Allocation> and Into<Allocation> round trip handles manually allocating
/// and deallocating an internal vector that is shared across host/guest
/// If the AllocationPtr was to be cloned the shared vector could be allocated and
/// deallocated in an undefined way
pub struct AllocationPtr(Ptr);

impl From<Ptr> for AllocationPtr {
    fn from(ptr: Ptr) -> AllocationPtr {
        AllocationPtr(ptr)
    }
}

impl AllocationPtr {
    /// normally we don't want to expose the inner Ptr because cloning or reusing it
    /// can lead to bad allocation and deallocation
    /// in the case that we pass the inner Ptr over to the host we do need to access it
    /// this function is just like From<AllocationPtr> for Ptr but renamed to make it
    /// clear that it has only one valid use-case
    pub fn as_remote_ptr(&self) -> RemotePtr {
        self.0
    }

    pub fn from_remote_ptr(host_ptr: RemotePtr) -> Self {
        Self(host_ptr)
    }
}
