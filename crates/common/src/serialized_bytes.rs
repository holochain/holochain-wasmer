use crate::allocation;
use crate::allocation::Allocation;
use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use holochain_serialized_bytes::prelude::*;

pub fn from_allocation_ptr(allocation_ptr: AllocationPtr) -> SerializedBytes {
    let allocation = allocation::from_allocation_ptr(allocation_ptr);
    let b: Vec<u8> =
        unsafe { std::slice::from_raw_parts(allocation[0] as _, allocation[1] as _) }.into();
    println!("{:?}", allocation);
    println!("{} {}", b.as_ptr() as Ptr, b.len());
    SerializedBytes::from(UnsafeBytes::from(b))
}

pub fn to_allocation_ptr(sb: SerializedBytes) -> AllocationPtr {
    let bytes: Vec<u8> = UnsafeBytes::from(sb).into();
    let bytes_ptr = bytes.as_ptr() as Ptr;
    let bytes_len = bytes.len() as Len;
    std::mem::ManuallyDrop::new(bytes);
    let allocation: Allocation = [bytes_ptr, bytes_len];
    allocation::to_allocation_ptr(allocation)
}

#[cfg(test)]
pub mod tests {
    use crate::serialized_bytes;
    use holochain_serialized_bytes::prelude::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Foo(String);

    holochain_serial!(Foo);

    #[test]
    fn serialized_bytes_from_allocation_test() {
        let foo: Foo = Foo("foo".into());
        let foo_sb: SerializedBytes = foo.clone().try_into().unwrap();

        let ptr = serialized_bytes::to_allocation_ptr(foo_sb.clone());
        let recovered_foo_sb = serialized_bytes::from_allocation_ptr(ptr);

        // can't do it twice
        // let second_foo_sb = serialized_bytes::from_allocation_ptr(ptr);

        assert_eq!(foo_sb, recovered_foo_sb);
        // assert_ne!(foo_sb, second_foo_sb);

        let recovered_foo: Foo = recovered_foo_sb.try_into().unwrap();

        assert_eq!(foo, recovered_foo);
    }
}
