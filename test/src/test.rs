pub mod import;
pub mod wasms;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;
use wasmer::FunctionEnvMut;
use wasmer_middlewares::metering::MeteringPoints;

pub fn short_circuit(
    _env: FunctionEnvMut<Env>,
    _: GuestPtr,
    _: Len,
) -> Result<u64, wasmer::RuntimeError> {
    Err(wasm_error!(WasmErrorInner::HostShortCircuit(
        holochain_serialized_bytes::encode(&String::from("shorts")).map_err(|e| wasm_error!(e))?,
    ))
    .into())
}

pub fn test_process_string(
    mut function_env: FunctionEnvMut<Env>,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer::RuntimeError> {
    let (env, mut store_mut) = function_env.data_and_store_mut();
    let string: String = env.consume_bytes_from_guest(&mut store_mut, guest_ptr, len)?;
    let processed_string = format!("host: {}", string);
    env.move_data_to_guest(&mut store_mut, Ok::<String, WasmError>(processed_string))
}

pub fn test_process_struct(
    mut function_env: FunctionEnvMut<Env>,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer::RuntimeError> {
    let (env, mut store_mut) = function_env.data_and_store_mut();
    let mut some_struct: SomeStruct =
        env.consume_bytes_from_guest(&mut store_mut, guest_ptr, len)?;
    some_struct.process();
    env.move_data_to_guest(&mut store_mut, Ok::<SomeStruct, WasmError>(some_struct))
}

pub fn debug(
    mut function_env: FunctionEnvMut<Env>,
    some_number: i32,
) -> Result<(), wasmer::RuntimeError> {
    let (env, mut store_mut) = function_env.data_and_store_mut();
    println!("debug {:?}", some_number);
    env.move_data_to_guest(&mut store_mut, ())?;
    Ok(())
}

pub fn decrease_points(
    mut function_env: FunctionEnvMut<Env>,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer::RuntimeError> {
    let (env, mut store_mut) = function_env.data_and_store_mut();
    let points: u64 = env.consume_bytes_from_guest(&mut store_mut, guest_ptr, len)?;
    let points_before = env.get_remaining_points(&mut store_mut)?;
    let remaining_points = env.decrease_points(&mut store_mut, points)?;
    let points_after = env.get_remaining_points(&mut store_mut)?;
    assert_eq!(points_after, remaining_points);
    env.move_data_to_guest(
        &mut store_mut,
        Ok::<(u64, u64), WasmError>(match (points_before, remaining_points) {
            (
                MeteringPoints::Remaining(points_before),
                MeteringPoints::Remaining(remaining_points),
            ) => (points_before, remaining_points),
            // This will error on the guest because it will require at least 1 point
            // to deserialize this value.
            _ => (0, 0),
        }),
    )
}

pub fn err(_: FunctionEnvMut<Env>) -> Result<(), wasmer::RuntimeError> {
    Err(wasm_error!(WasmErrorInner::Guest("oh no!".into())).into())
}

pub fn pages(
    mut function_env: FunctionEnvMut<Env>,
    _: WasmSize,
) -> Result<WasmSize, wasmer::RuntimeError> {
    let (env, store_mut) = function_env.data_and_store_mut();
    Ok(env
        .memory
        .as_ref()
        .ok_or(wasm_error!(WasmErrorInner::Memory))?
        .view(&store_mut)
        .size()
        .0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::wasms;
    use holochain_wasmer_host::module::InstanceWithStore;
    use std::thread;
    use test_common::StringType;
    use wasmer::AsStoreMut;
    use wasms::TestWasm;

    #[ctor::ctor]
    fn before() {
        env_logger::init();
    }

    #[test]
    fn host_externs_toolable() {
        let module = (*TestWasm::Test.module(false)).clone();
        // Imports will be the minimal set of functions actually used by the wasm
        // NOT the complete list defined by `host_externs!`.
        assert_eq!(
            vec![
                "__hc__short_circuit_5".to_string(),
                "__hc__test_process_string_2".to_string(),
                "__hc__test_process_struct_2".to_string(),
                "__hc__decrease_points_1".to_string(),
            ],
            module
                .imports()
                .map(|import| import.name().to_string())
                .collect::<Vec<String>>()
        );
    }

    // Reinstate this test when metering is working.
    #[test]
    #[ignore]
    fn infinite_loop() {
        // Instead of looping forever we want the metering to kick in and trap
        // the execution into an unreachable error.
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: Result<(), _> = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "loop_forever",
            (),
        );
        assert!(result.is_err());
    }

    #[test]
    fn short_circuit() {
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: String = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "short_circuit",
            (),
        )
        .unwrap();
        assert_eq!(result, String::from("shorts"));
    }

    #[test]
    fn bytes_round_trip() {
        let InstanceWithStore { store, instance } = TestWasm::Memory.instance();
        let _: () = dbg!(guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "bytes_round_trip",
            ()
        ))
        .unwrap();
    }

    #[test]
    fn stacked_test() {
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: String = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "stacked_strings",
            (),
        )
        .expect("stacked strings call");

        assert_eq!("first", &result);
    }

    #[test]
    fn literal_bytes() {
        let input: Vec<u8> = vec![1, 2, 3];
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: Vec<u8> = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "literal_bytes",
            input.clone(),
        )
        .expect("literal_bytes call");
        assert_eq!(input, result);
    }

    #[test]
    fn ignore_args_process_string_test() {
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: StringType = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "ignore_args_process_string",
            &StringType::from(String::new()),
        )
        .expect("ignore_args_process_string call");
        assert_eq!(String::new(), String::from(result));
    }

    // https://github.com/trailofbits/test-fuzz/issues/171
    #[cfg(not(target_os = "windows"))]
    #[test_fuzz::test_fuzz]
    fn process_string_fuzz(s: String) {
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: StringType = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "process_string",
            &StringType::from(s.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", s);

        assert_eq!(&String::from(result), &expected_string);
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯"
            .repeat(usize::try_from(10_u32 * u32::try_from(std::u16::MAX).unwrap()).unwrap());
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let result: StringType = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "process_string",
            // This is by reference just to show that it can be done as borrowed or owned.
            &StringType::from(starter_string.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(&String::from(result), &expected_string,);
    }

    #[test]
    fn concurrent_calls() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let InstanceWithStore {
            store: store_1,
            instance: instance_1,
        } = TestWasm::Test.instance();
        let InstanceWithStore {
            store: store_2,
            instance: instance_2,
        } = TestWasm::Test.instance();

        let call_1 = thread::spawn({
            let some_struct = some_struct.clone();
            move || {
                guest::call::<_, SomeStruct>(
                    &mut store_1.lock().as_store_mut(),
                    instance_1,
                    "native_type",
                    some_struct.clone(),
                )
            }
        });
        let call_2 = thread::spawn({
            let some_struct = some_struct.clone();
            move || {
                guest::call::<_, SomeStruct>(
                    &mut store_2.lock().as_store_mut(),
                    instance_2,
                    "native_type",
                    some_struct.clone(),
                )
            }
        });
        assert!(matches!(call_1.join(), Ok(SomeStruct)));
        assert!(matches!(call_2.join(), Ok(SomeStruct)));
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let InstanceWithStore { store, instance } = TestWasm::Test.instance();

        let result: SomeStruct = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "native_type",
            some_struct.clone(),
        )
        .expect("native type handling");

        assert_eq!(some_struct, result);
    }

    #[test]
    fn native_struct_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let InstanceWithStore { store, instance } = TestWasm::Test.instance();

        let result: SomeStruct = guest::call(
            &mut store.lock().as_store_mut(),
            instance,
            "process_native",
            some_struct.clone(),
        )
        .unwrap();

        let expected = SomeStruct::new(format!("processed: {}", some_inner));
        assert_eq!(result, expected,);
    }

    #[test]
    fn ret_test() {
        let InstanceWithStore {
            store: store_foo,
            instance: instance_foo,
        } = TestWasm::Test.instance();

        let some_struct: SomeStruct = guest::call(
            &mut store_foo.lock().as_store_mut(),
            instance_foo,
            "some_ret",
            (),
        )
        .unwrap();
        assert_eq!(SomeStruct::new("foo".into()), some_struct,);

        let InstanceWithStore {
            store: store_ret_err,
            instance: instance_ret_err,
        } = TestWasm::Test.instance();

        let err: Result<SomeStruct, wasmer::RuntimeError> = guest::call(
            &mut store_ret_err.lock().as_store_mut(),
            instance_ret_err,
            "some_ret_err",
            (),
        );
        match err {
            Err(runtime_error) => assert_eq!(
                WasmError {
                    file: "src/wasm.rs".into(),
                    line: 101,
                    error: WasmErrorInner::Guest("oh no!".into()),
                },
                runtime_error.downcast().unwrap(),
            ),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_ptr_test() {
        let InstanceWithStore {
            store: store_succeed,
            instance: instance_succeed,
        } = TestWasm::Test.instance();

        let success_result: Result<SomeStruct, ()> = guest::call(
            &mut store_succeed.lock().as_store_mut(),
            instance_succeed,
            "try_ptr_succeeds",
            (),
        )
        .unwrap();
        assert_eq!(SomeStruct::new("foo".into()), success_result.unwrap());

        let InstanceWithStore {
            store: store_fail,
            instance: instance_fail,
        } = TestWasm::Test.instance();

        let fail_result: Result<(), wasmer::RuntimeError> = guest::call(
            &mut store_fail.lock().as_store_mut(),
            instance_fail,
            "try_ptr_fails_fast",
            (),
        );
        match fail_result {
            Err(runtime_error) => {
                assert_eq!(
                    WasmError {
                        file: "src/wasm.rs".into(),
                        line: 129,
                        error: WasmErrorInner::Guest("it fails!: ()".into()),
                    },
                    runtime_error.downcast().unwrap(),
                );
            }
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn decrease_points_test() {
        let InstanceWithStore { store, instance } = TestWasm::Test.instance();
        let dec_by = 1_000_000_u64;
        let points_before: u64 = instance
            .exports
            .get_global("wasmer_metering_remaining_points")
            .unwrap()
            .get(&mut store.lock().as_store_mut())
            .unwrap_i64()
            .try_into()
            .unwrap();

        let (before_decrease, after_decrease): (u64, u64) = guest::call(
            &mut store.lock().as_store_mut(),
            instance.clone(),
            "decrease_points",
            dec_by,
        )
        .unwrap();

        let points_after: u64 = instance
            .exports
            .get_global("wasmer_metering_remaining_points")
            .unwrap()
            .get(&mut store.lock().as_store_mut())
            .unwrap_i64()
            .try_into()
            .unwrap();

        assert!(before_decrease - after_decrease == dec_by);
        assert!(
            points_before > before_decrease
                && before_decrease > after_decrease
                && after_decrease > points_after
        );
    }
}
