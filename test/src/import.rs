use crate::debug;
use crate::test_process_string;
use crate::test_process_struct;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::ImportObject;

pub fn memory_only() -> ImportObject {
    imports! {
        "env" => {
            "__import_allocation" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_allocation),
            "__import_bytes" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_bytes),
        },
    }
}

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__import_allocation" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_allocation),
            "__import_bytes" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_bytes),
            "__test_process_string" => func!(test_process_string),
            "__test_process_struct" => func!(test_process_struct),
            "__debug" => func!(debug),
        },
    }
}
