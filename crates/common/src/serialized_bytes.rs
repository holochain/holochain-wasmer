use crate::allocation::Allocation;
use crate::AllocationPtr;
use crate::Len;
use crate::Ptr;
use holochain_serialized_bytes::prelude::*;

impl From<AllocationPtr> for SerializedBytes {
    fn from(allocation_ptr: AllocationPtr) -> SerializedBytes {
        let allocation = Allocation::from(allocation_ptr);
        let b: Vec<u8> = unsafe {
            Vec::from_raw_parts(allocation[0] as _, allocation[1] as _, allocation[1] as _)
        };
        SerializedBytes::from(UnsafeBytes::from(b))
    }
}

impl From<SerializedBytes> for AllocationPtr {
    fn from(sb: SerializedBytes) -> AllocationPtr {
        let bytes: Vec<u8> = UnsafeBytes::from(sb).into();
        let bytes_ptr = bytes.as_ptr() as Ptr;
        let bytes_len = bytes.len() as Len;
        std::mem::ManuallyDrop::new(bytes);
        let allocation: Allocation = [bytes_ptr, bytes_len];
        AllocationPtr::from(allocation)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::AllocationPtr;
    use holochain_serialized_bytes::prelude::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Foo(String);

    holochain_serial!(Foo);

    #[test]
    fn serialized_bytes_from_allocation_test() {
        let foo: Foo = Foo("foo".into());
        let foo_sb: SerializedBytes = foo.clone().try_into().unwrap();

        let ptr: AllocationPtr = foo_sb.clone().into();
        let recovered_foo_sb: SerializedBytes = ptr.into();

        // can't do it twice
        // let second_foo_sb = serialized_bytes::from_allocation_ptr(ptr);

        assert_eq!(foo_sb, recovered_foo_sb);
        // assert_ne!(foo_sb, second_foo_sb);

        let recovered_foo: Foo = recovered_foo_sb.try_into().unwrap();

        assert_eq!(foo, recovered_foo);
    }
}
