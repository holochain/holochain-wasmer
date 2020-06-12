pub mod allocation;
pub mod result;

pub use holochain_serialized_bytes::prelude::*;
pub use result::*;

use crate::allocation::Allocation;
use crate::allocation::ALLOCATION_ITEMS;

pub type Ptr = u64;
pub type Len = u64;

pub type GuestPtr = Ptr;

/// AllocationPtr wraps a ptr that is used to pass the location of an Allocation
/// between the host and guest (in either direction).
/// The AllocationPtr intentionally does not implement Clone
/// The From<Allocation> and Into<Allocation> round trip handles manually allocating
/// and deallocating an internal vector that is shared across host/guest
/// If the AllocationPtr was to be cloned the shared vector could be allocated and
/// deallocated in an undefined way
pub struct AllocationPtr(Ptr);

impl AllocationPtr {
    /// normally we don't want to expose the inner Ptr because cloning or reusing it
    /// can lead to bad allocation and deallocation
    /// in the case that we pass the inner Ptr over to the host we do need to access it
    /// this function is just like From<AllocationPtr> for Ptr but renamed to make it
    /// clear that it has only one valid use-case
    pub fn as_guest_ptr(&self) -> GuestPtr {
        self.0
    }

    pub fn from_guest_ptr(guest_ptr: GuestPtr) -> Self {
        Self(guest_ptr)
    }

    /// get the Allocation for this Allocation _without_ deallocating the Allocation in the process
    /// usually you do not want to do this because From<AllocationPtr> for Allocation consumes the
    /// original AllocationPtr and returns a new identical Allocation
    pub fn peek_allocation(&self) -> Allocation {
        let allocation_slice: &[u64] =
            unsafe { std::slice::from_raw_parts(self.0 as _, ALLOCATION_ITEMS) };
        [allocation_slice[0], allocation_slice[1]]
    }
}
