pub mod import;
pub mod wasms;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;

pub fn short_circuit(_: &Env, _: GuestPtr, _: Len) -> Result<u64, wasmer_engine::RuntimeError> {
    Err(wasm_error!(WasmErrorInner::HostShortCircuit(
        holochain_serialized_bytes::encode(&String::from("shorts"))
            .map_err(|e| wasm_error!(e.into()))?,
    ))
    .into())
}

pub fn test_process_string(
    env: &Env,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer_engine::RuntimeError> {
    let string: String = env.consume_bytes_from_guest(guest_ptr, len)?;
    let processed_string = format!("host: {}", string);
    Ok(env.move_data_to_guest(Ok::<String, WasmError>(processed_string))?)
}

pub fn test_process_struct(
    env: &Env,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer_engine::RuntimeError> {
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
    some_struct.process();
    Ok(env.move_data_to_guest(Ok::<SomeStruct, WasmError>(some_struct))?)
}

pub fn debug(env: &Env, some_number: WasmSize) -> Result<u64, wasmer_engine::RuntimeError> {
    println!("debug {:?}", some_number);
    Ok(env.move_data_to_guest(())?)
}

pub fn pages(env: &Env, _: WasmSize) -> Result<WasmSize, wasmer_engine::RuntimeError> {
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
    use holochain_wasmer_common::scopetracker::ScopeTracker;
    use test_common::StringType;
    use wasms::TestWasm;

    #[test]
    fn short_circuit() {
        let result: String = guest::call(TestWasm::Test.instance(), "short_circuit", ()).unwrap();
        assert_eq!(result, String::from("shorts"));
    }

    #[test]
    fn bytes_round_trip() {
        let _: () = guest::call(TestWasm::Memory.instance(), "bytes_round_trip", ()).unwrap();
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

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);
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

        let err: Result<SomeStruct, wasmer_engine::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "some_ret_err", ());
        match err {
            Err(runtime_error) => assert_eq!(
                WasmError {
                    file: "src/wasm.rs".into(),
                    line: 103,
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

        let fail_result: Result<(), wasmer_engine::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "try_ptr_fails_fast", ());

        match fail_result {
            Err(runtime_error) => {
                assert_eq!(
                    WasmError {
                        file: "src/wasm.rs".into(),
                        line: 132,
                        error: WasmErrorInner::Guest("it fails!: ()".into()),
                    },
                    runtime_error.downcast().unwrap(),
                );
            }
            Ok(_) => unreachable!(),
        };
    }

    // FIXME: on macos, the leak detection doesn't work reliably
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn mem_leak() {
        let mut leaked = vec![];
        let mut leaked_workaround = vec![];

        let input = test_common::StringType::from(String::new());

        #[derive(Debug)]
        #[allow(dead_code)]
        struct Leaked {
            runs: usize,
            num_threads: usize,
            bytes: isize,
            // kb: isize,
            mb: f64,
            workaround_leak: bool,
        }

        let outer_guard = mem_guard!("test::mem_leak::outer");
        let _instance = TestWasm::Test.instance();

        for num_thread in &[20] {
            for runs in &[100, 400] {
                for workaround_leak in &[true, false] {
                    let guard = mem_guard!("test::mem_leak::inner");

                    for _ in 0..*runs {
                        {
                            if !*workaround_leak {
                                TestWasm::impair_leak_workaround();
                            }

                            let mut threads = vec![];

                            for _ in 0..*num_thread {
                                let input = input.clone();
                                threads.push(std::thread::spawn(move || {
                                    let _: test_common::StringType =
                                        holochain_wasmer_host::guest::call(
                                            TestWasm::Test.instance(),
                                            "process_string",
                                            &input,
                                        )
                                        .unwrap();
                                }));
                            }

                            for thread in threads {
                                thread.join().unwrap();
                            }
                        }
                    }

                    let leaked_bytes = guard.leaked();
                    let leaked_struct = Leaked {
                        runs: *runs,
                        num_threads: *num_thread,
                        bytes: leaked_bytes,
                        mb: leaked_bytes as f64 / 1_000_000.0,
                        workaround_leak: *workaround_leak,
                    };
                    println!("{:?}", leaked_struct);

                    if *workaround_leak {
                        leaked_workaround.push(leaked_struct);
                    } else {
                        leaked.push(leaked_struct);
                    }

                    TestWasm::reset_module_cache();
                }
            }
        }

        TestWasm::reset_module_cache();

        println!(
            "overall leaked despite module cache reset: {}",
            outer_guard.leaked() as f64 / 1_000_000.0
        );

        let threshold = 20.0;
        let max_leak_with_workaround = leaked_workaround
            .iter()
            .max_by(|a, b| a.bytes.cmp(&b.bytes))
            .unwrap()
            .mb;

        assert!(
            max_leak_with_workaround < threshold,
            "expected all cases with the workaround to leak less than {}mb",
            threshold
        );

        // on windows the leak seems to be less severe
        // FIXME: on macos, the leak detection doesn't work reliably
        #[cfg(target_os = "linux")]
        assert!(
            leaked
                .iter()
                .min_by(|a, b| a.bytes.cmp(&b.bytes))
                .unwrap()
                .mb
                > threshold,
            "expected all cases without the workaround to leak more than {}mb",
            threshold
        );
    }
}
