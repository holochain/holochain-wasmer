use crate::bytes;
use crate::AllocationPtr;

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> String {
    let bytes = bytes::from_allocation_ptr(allocation_ptr);
    String::from(unsafe { std::str::from_utf8_unchecked(&bytes) })
}

pub fn to_allocation_ptr(s: String) -> AllocationPtr {
    bytes::to_allocation_ptr(s.into_bytes())
}

#[cfg(test)]
pub mod tests {
    use crate::string;

    #[test]
    fn string_from_allocation_test() {
        let some_string = String::from("foo");

        let ptr = string::to_allocation_ptr(some_string.clone());
        let recovered_string = string::from_allocation_ptr(ptr);

        assert_eq!(some_string, recovered_string,);
    }
}
