use crate::test_process_string;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::ImportObject;

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__import_allocation" => wasmer_runtime::func!(host::allocation::write_to_guest),
            "__import_bytes" => wasmer_runtime::func!(host::bytes::write_to_guest_using_host_allocation_ptr),
            "__test_process_string" => func!(test_process_string),
        },
    }
}
