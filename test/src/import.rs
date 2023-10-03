// use crate::debug;
use crate::err;
// use crate::pages;
// use crate::short_circuit;
// use crate::test_process_string;
// use crate::test_process_struct;
use holochain_wasmer_host::prelude::*;
use wasmer::imports;
use wasmer::AsStoreMut;
use wasmer::Function;
use wasmer::FunctionEnv;
use wasmer::FunctionEnvMut;
use wasmer::Imports;

pub fn debug(_env: FunctionEnvMut<Env>, some_number: i32) -> i32 {
    println!("debug {:?}", some_number);
    // env.move_data_to_guest(())
    0
}

/// ```
/// # use wasmer::{Store, Function, FunctionEnv, FunctionEnvMut};
/// # let mut store = Store::default();
/// # let env = FunctionEnv::new(&mut store, ());
/// #
/// fn sum(_env: FunctionEnvMut<()>, a: i32, b: i32) -> i32 {
///     a + b
/// }
///
/// let f = Function::new_typed_with_env(&mut store, &env, sum);
/// ```

pub fn imports(store: &mut impl AsStoreMut, env: Env) -> Imports {
    let function_env = FunctionEnv::new(store, env);
    imports! {
        "env" => {
            // "__hc__short_circuit_5" => Function::new_typed_with_env(
            //     store,
            //     env.clone(),
            //     short_circuit
            // ),
            // "__hc__test_process_string_2" => Function::new_typed_with_env(
            //     store,
            //     env.clone(),
            //     test_process_string
            // ),
            // "__hc__test_process_struct_2" => Function::new_typed_with_env(
            //     store,
            //     env.clone(),
            //     test_process_struct
            // ),
            "__hc__debug_1" => Function::new_typed_with_env(
                store,
                &function_env,
                debug
            ),
            "__hc__guest_err_1" => Function::new_typed_with_env(
                store,
                &function_env,
                err
            ),
            // "__hc__pages_1" => Function::new_typed_with_env(
            //     store,
            //     env.clone(),
            //     pages
            // ),
        },
    }
}
