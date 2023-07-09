use crate::debug;
use crate::pages;
use crate::short_circuit;
use crate::test_process_string;
use crate::test_process_struct;
use holochain_wasmer_host::prelude::*;

pub fn import_object(store: &Store, env: &Env) -> ImportObject {
    imports! {
        "env" => {
            "__short_circuit" => Function::new_native_with_env(
                store,
                env.clone(),
                short_circuit
            ),
            "__test_process_string" => Function::new_native_with_env(
                store,
                env.clone(),
                test_process_string
            ),
            "__test_process_struct" => Function::new_native_with_env(
                store,
                env.clone(),
                test_process_struct
            ),
            "__debug" => Function::new_native_with_env(
                store,
                env.clone(),
                debug
            ),
            "__pages" => Function::new_native_with_env(
                store,
                env.clone(),
                pages
            ),
        },
    }
}
