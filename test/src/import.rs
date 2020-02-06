use crate::test_process_string;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::ImportObject;

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__copy_allocation_to_guest" => wasmer_runtime::func!(host::allocate::copy_allocation_to_guest),
            "__host_copy_string" => wasmer_runtime::func!(host::host_copy_string),
            "__test_process_string" => func!(test_process_string),
        },
    }
}
