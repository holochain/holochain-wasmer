use wasmer_runtime::imports;
use wasmer_runtime::func;
use wasmer_runtime::ImportObject;
use crate::host_process_string;
use crate::host_copy_string;
use crate::allocate::copy_allocation_to_guest;

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__host_process_string" => func!(host_process_string),
            "__copy_allocation_to_guest" => func!(copy_allocation_to_guest),
            "__host_copy_string" => func!(host_copy_string),
        },
    }
}
