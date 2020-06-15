use crate::debug;
use crate::pages;
use crate::test_process_string;
use crate::test_process_struct;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::ImportObject;

pub fn memory_only() -> ImportObject {
    imports! {
        "env" => {
            "__import_data" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_data),
        },
    }
}

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__import_data" => wasmer_runtime::func!(holochain_wasmer_host::import::__import_data),
            "__test_process_string" => func!(test_process_string),
            "__test_process_struct" => func!(test_process_struct),
            "__debug" => func!(debug),
            "__pages" => func!(pages),
        },
    }
}
