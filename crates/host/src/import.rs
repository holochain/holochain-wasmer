use crate::guest;
use crate::prelude::*;
use wasmer_runtime::Ctx;

pub fn __import_data(ctx: &mut Ctx, guest_ptr: GuestPtr) {
    if ctx.data == 0 as _ {
        unreachable!();
    }
    let host_allocation = AllocationPtr::from_guest_ptr(ctx.data as _);
    let host_sb: SerializedBytes = host_allocation.into();
    ctx.data = 0 as _;
    let guest_allocation = guest::allocation_from_guest_ptr(ctx, guest_ptr).unwrap();
    guest::write_slice(ctx, guest_allocation[0], host_sb.bytes());
}
