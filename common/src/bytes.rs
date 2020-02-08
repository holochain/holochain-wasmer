use crate::allocation;
use crate::allocation::Allocation;
use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use std::mem;
use std::slice;

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> Vec<u8> {
    let allocation = allocation::from_allocation_ptr(allocation_ptr);
    unsafe { slice::from_raw_parts(allocation[0] as _, allocation[1] as _) }.into()
}

pub fn to_allocation_ptr(bytes: Vec<u8>) -> AllocationPtr {
    let bytes_ptr = bytes.as_ptr() as Ptr;
    let bytes_len = bytes.len() as Len;
    mem::ManuallyDrop::new(bytes);
    let allocation: Allocation = [bytes_ptr, bytes_len];
    allocation::to_allocation_ptr(allocation)
}

#[cfg(test)]
pub mod tests {
    use crate::bytes;

    #[test]
    fn bytes_from_allocation_test() {
        let some_string = String::from("foo");

        let ptr = bytes::to_allocation_ptr(some_string.clone().into_bytes());
        let recovered_bytes = bytes::from_allocation_ptr(ptr);
        let recovered_string =
            String::from(std::str::from_utf8(&recovered_bytes).expect("bad utf8"));

        assert_eq!(some_string, recovered_string,);
    }
}
