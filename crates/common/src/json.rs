use crate::bytes;
use crate::AllocationPtr;
use holochain_json_api::json::JsonString;

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> JsonString {
    let b = bytes::from_allocation_ptr(allocation_ptr);
    JsonString::from_bytes(b)
}

pub fn to_allocation_ptr(j: JsonString) -> AllocationPtr {
    bytes::to_allocation_ptr(j.to_bytes())
}

#[cfg(test)]
pub mod tests {
    use crate::json;
    use holochain_json_api::json::JsonString;

    #[test]
    fn json_from_allocation_test() {
        let some_json = JsonString::from_json("\"foo\"");

        let ptr = json::to_allocation_ptr(some_json.clone());
        let recovered_json = json::from_allocation_ptr(ptr);

        assert_eq!(some_json, recovered_json);
    }
}
