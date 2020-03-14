use crate::bytes;
use crate::AllocationPtr;
use holochain_serialized_bytes::prelude::*;

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> SerializedBytes {
    let b = bytes::from_allocation_ptr(allocation_ptr);
    SerializedBytes::from(UnsafeBytes::from(b))
}

pub fn to_allocation_ptr(sb: SerializedBytes) -> AllocationPtr {
    bytes::to_allocation_ptr(sb.bytes().to_vec())
}

#[cfg(test)]
pub mod tests {
    use crate::serialized_bytes;
    use holochain_serialized_bytes::prelude::*;

    #[derive(Serialize, Deserialize)]
    struct Foo(String);

    holochain_serial!(Foo, Bar);

    #[test]
    fn json_from_allocation_test() {
        let foo: Foo = Foo("foo".into());
        let foo_sb: SerializedBytes = foo.clone().try_into().unwrap();

        let ptr = serialized_bytes::to_allocation_ptr(foo_sb.clone());
        let recovered_foo_sb = serialized_bytes::from_allocation_ptr(ptr);

        assert_eq!(foo_sb, recovered_foo_sb);

        let recovered_foo: Foo = recovered_foo_sb.try_into().unwrap();

        assert_eq!(foo, recovered_foo);
    }
}
