use crate::test_process_string;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::ImportObject;

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__import_allocation" => wasmer_runtime::func!(host::import::__import_allocation),
            "__import_bytes" => wasmer_runtime::func!(host::import::__import_bytes),
            "__test_process_string" => func!(test_process_string),
        },
    }
}
