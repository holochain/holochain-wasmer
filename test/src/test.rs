pub mod import;
pub mod wasms;

#[cfg(test)]
use crate::scopetracker::ScopeTracker;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;

pub fn short_circuit(_: &Env, _: GuestPtr, _: Len) -> Result<(), WasmError> {
    RuntimeError::raise(Box::new(WasmError::HostShortCircuit(
        holochain_serialized_bytes::encode(&String::from("shorts"))?,
    )));
}

pub fn test_process_string(env: &Env, guest_ptr: GuestPtr, len: Len) -> Result<(), WasmError> {
    let string: String = env.consume_bytes_from_guest(guest_ptr, len)?;
    let processed_string = format!("host: {}", string);
    Ok(env.set_data(Ok::<String, WasmError>(processed_string))?)
}

pub fn test_process_struct(env: &Env, guest_ptr: GuestPtr, len: Len) -> Result<(), WasmError> {
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
    some_struct.process();
    Ok(env.set_data(Ok::<SomeStruct, WasmError>(some_struct))?)
}

pub fn debug(_env: &Env, some_number: WasmSize) -> Result<(), WasmError> {
    println!("debug {:?}", some_number);
    Ok(())
}

pub fn pages(env: &Env, _: WasmSize) -> Result<WasmSize, WasmError> {
    Ok(env.memory_ref().ok_or(WasmError::Memory)?.size().0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::wasms;
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

        let err: Result<SomeStruct, WasmError> =
            guest::call(TestWasm::Test.instance(), "some_ret_err", ());
        match err {
            Err(wasm_error) => assert_eq!(WasmError::Guest("oh no!".into()), wasm_error,),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_ptr_test() {
        let success_result: Result<SomeStruct, ()> =
            guest::call(TestWasm::Test.instance(), "try_ptr_succeeds", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), success_result.unwrap());

        let fail_result: Result<(), WasmError> =
            guest::call(TestWasm::Test.instance(), "try_ptr_fails_fast", ());

        match fail_result {
            Err(wasm_error) => {
                assert_eq!(WasmError::Guest("it fails!: ()".into()), wasm_error,);
            }
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn mem_leak() {
        let mut leaked = std::collections::HashMap::<(usize, usize), isize>::new();

        let input = test_common::StringType::from(".".repeat(0));

        let _instance = TestWasm::Test.instance();
        for n in &[1, 25] {
            for m in &[1, 25, 100, 1000] {
                let guard = mem_guard!("test::mem_leak");

                for _ in 0..*m {
                    {
                        let mut threads = vec![];

                        for _ in 0..*n {
                            let instance = TestWasm::Test.instance();
                            let input = input.clone();
                            threads.push(std::thread::spawn(move || {
                                let _: test_common::StringType =
                                    holochain_wasmer_host::guest::call(
                                        instance,
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
                leaked.insert((*n, *m), guard.leaked());
            }
        }

        let mut leaked = leaked
            .into_iter()
            .map(|mut l| {
                l.1 /= 1_000_000;
                l
            })
            .collect::<Vec<_>>();
        leaked.sort_by_key(|l| l.1);
        assert!(false, "{:#?}", leaked);
    }
}
