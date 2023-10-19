use crate::debug;
use crate::err;
use crate::pages;
use crate::short_circuit;
use crate::test_process_string;
use crate::test_process_struct;
use holochain_wasmer_host::prelude::*;
use wasmer::imports;
use wasmer::Function;
use wasmer::FunctionEnv;
use wasmer::Imports;
use wasmer::StoreMut;

pub fn imports(store: &mut StoreMut, function_env: &FunctionEnv<Env>) -> Imports {
    imports! {
        "env" => {
            "__hc__short_circuit_5" => Function::new_typed_with_env(
                store,
                function_env,
                short_circuit
            ),
            "__hc__test_process_string_2" => Function::new_typed_with_env(
                store,
                function_env,
                test_process_string
            ),
            "__hc__test_process_struct_2" => Function::new_typed_with_env(
                store,
                function_env,
                test_process_struct
            ),
            "__hc__debug_1" => Function::new_typed_with_env(
                store,
                function_env,
                debug
            ),
            "__hc__guest_err_1" => Function::new_typed_with_env(
                store,
                function_env,
                err
            ),
            "__hc__pages_1" => Function::new_typed_with_env(
                store,
                function_env,
                pages
            ),
        },
    }
}
