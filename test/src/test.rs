pub mod import;
pub mod wasms;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;
use wasmer::FunctionEnv;

pub fn short_circuit(
    env: FunctionEnv<Env>,
    _: GuestPtr,
    _: Len,
) -> Result<u64, wasmer::RuntimeError> {
    Err(wasm_error!(WasmErrorInner::HostShortCircuit(
        holochain_serialized_bytes::encode(&String::from("shorts")).map_err(|e| wasm_error!(e))?,
    ))
    .into())
}

pub fn test_process_string(
    env: FunctionEnv<Env>,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer::RuntimeError> {
    let string: String = env.consume_bytes_from_guest(guest_ptr, len)?;
    let processed_string = format!("host: {}", string);
    env.move_data_to_guest(Ok::<String, WasmError>(processed_string))
}

pub fn test_process_struct(
    env: FunctionEnv<Env>,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer::RuntimeError> {
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
    some_struct.process();
    env.move_data_to_guest(Ok::<SomeStruct, WasmError>(some_struct))
}

pub fn debug(env: FunctionEnv<Env>, some_number: i32) -> i32 {
    println!("debug {:?}", some_number);
    // env.move_data_to_guest(())
    0
}

pub fn pages(env: FunctionEnv<Env>, _: WasmSize) -> Result<WasmSize, wasmer::RuntimeError> {
    Ok(env
        .memory_ref()
        .ok_or(wasm_error!(WasmErrorInner::Memory))?
        .size()
        .0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::wasms;
    use test_common::StringType;
    use wasms::TestWasm;

    #[ctor::ctor]
    fn before() {
        env_logger::init();
    }

    #[test]
    fn host_externs_toolable() {
        let module = TestWasm::Test.module(false);
        // Imports will be the minimal set of functions actually used by the wasm
        // NOT the complete list defined by `host_externs!`.
        assert_eq!(
            vec![
                "__hc__short_circuit_5".to_string(),
                "__hc__test_process_string_2".to_string(),
                "__hc__test_process_struct_2".to_string()
            ],
            module
                .imports()
                .map(|import| import.name().to_string())
                .collect::<Vec<String>>()
        );
    }

    #[test]
    fn infinite_loop() {
        // Instead of looping forever we want the metering to kick in and trap
        // the execution into an unreachable error.
        let result: Result<(), _> = guest::call(TestWasm::Test.instance(), "loop_forever", ());
        assert!(result.is_err());
    }

    #[test]
    fn short_circuit() {
        let result: String = guest::call(TestWasm::Test.instance(), "short_circuit", ()).unwrap();
        assert_eq!(result, String::from("shorts"));
    }

    #[test]
    fn bytes_round_trip() {
        let _: () = dbg!(guest::call(
            TestWasm::Memory.instance(),
            "bytes_round_trip",
            ()
        ))
        .unwrap();
    }

    #[test]
    fn stacked_test() {
        let result: String = guest::call(TestWasm::Test.instance(), "stacked_strings", ())
            .expect("stacked strings call");

        assert_eq!("first", &result);
    }

    #[test]
    fn literal_bytes() {
        let input: Vec<u8> = vec![1, 2, 3];
        let result: Vec<u8> =
            guest::call(TestWasm::Test.instance(), "literal_bytes", input.clone())
                .expect("literal_bytes call");
        assert_eq!(input, result);
    }

    #[test]
    fn ignore_args_process_string_test() {
        let result: StringType = guest::call(
            TestWasm::Test.instance(),
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
        let result: StringType = guest::call(
            TestWasm::Test.instance(),
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
        let result: StringType = guest::call(
            TestWasm::Test.instance(),
            "process_string",
            // This is by reference just to show that it can be done as borrowed or owned.
            &StringType::from(starter_string.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(&String::from(result), &expected_string,);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct = guest::call(
            TestWasm::Test.instance(),
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

        let result: SomeStruct = guest::call(
            TestWasm::Test.instance(),
            "process_native",
            some_struct.clone(),
        )
        .unwrap();

        let expected = SomeStruct::new(format!("processed: {}", some_inner));
        assert_eq!(result, expected,);
    }

    #[test]
    fn ret_test() {
        let some_struct: SomeStruct =
            guest::call(TestWasm::Test.instance(), "some_ret", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), some_struct,);

        let err: Result<SomeStruct, wasmer::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "some_ret_err", ());
        match err {
            Err(runtime_error) => assert_eq!(
                WasmError {
                    file: "src/wasm.rs".into(),
                    line: 100,
                    error: WasmErrorInner::Guest("oh no!".into()),
                },
                runtime_error.downcast().unwrap(),
            ),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_ptr_test() {
        let success_result: Result<SomeStruct, ()> =
            guest::call(TestWasm::Test.instance(), "try_ptr_succeeds", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), success_result.unwrap());

        let fail_result: Result<(), wasmer::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "try_ptr_fails_fast", ());

        match fail_result {
            Err(runtime_error) => {
                assert_eq!(
                    WasmError {
                        file: "src/wasm.rs".into(),
                        line: 128,
                        error: WasmErrorInner::Guest("it fails!: ()".into()),
                    },
                    runtime_error.downcast().unwrap(),
                );
            }
            Ok(_) => unreachable!(),
        };
    }
}
