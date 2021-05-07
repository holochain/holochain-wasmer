pub mod import;
pub mod wasms;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;

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

    fn test_instance(wasm: &[u8]) -> Instance {
        let engine = JIT::new(Singlepass::new()).engine();
        let store = Store::new(&engine);
        let env = Env::default();
        let module = Module::new(&store, wasm).unwrap();
        let import_object: ImportObject = import::import_object(&store, &env);
        let instance = Instance::new(&module, &import_object).unwrap();
        instance
    }

    #[test]
    fn bytes_round_trip() {
        let mut instance = test_instance(wasms::MEMORY);

        let _: () = guest::call(&mut instance, "bytes_round_trip", ()).unwrap();
    }

    #[test]
    fn stacked_test() {
        let result: String = guest::call(&mut test_instance(wasms::TEST), "stacked_strings", ())
            .expect("stacked strings call");

        assert_eq!("first", &result);
    }

    #[test]
    fn literal_bytes() {
        let input: Vec<u8> = vec![1, 2, 3];
        let result: Vec<u8> = guest::call(
            &mut test_instance(wasms::TEST),
            "literal_bytes",
            input.clone(),
        )
        .expect("literal_bytes call");
        assert_eq!(input, result);
    }

    #[test]
    fn ignore_args_process_string_test() {
        let mut instance = test_instance(wasms::TEST);
        let result: StringType = guest::call(
            &mut instance,
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
        let mut instance = test_instance(wasms::TEST);
        let result: StringType = guest::call(
            &mut instance,
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
            &mut test_instance(wasms::TEST),
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
            &mut test_instance(wasms::TEST),
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
            guest::call(&mut test_instance(wasms::TEST), "some_ret", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), some_struct,);

        let err: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(wasms::TEST), "some_ret_err", ());
        match err {
            Err(wasm_error) => assert_eq!(WasmError::Guest("oh no!".into()), wasm_error,),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_ptr_test() {
        let success_result: Result<SomeStruct, ()> =
            guest::call(&mut test_instance(wasms::TEST), "try_ptr_succeeds", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), success_result.unwrap());

        let fail_result: Result<(), WasmError> =
            guest::call(&mut test_instance(wasms::TEST), "try_ptr_fails_fast", ());

        match fail_result {
            Err(wasm_error) => {
                assert_eq!(WasmError::Guest("it fails!: ()".into()), wasm_error,);
            }
            Ok(_) => unreachable!(),
        };
    }
}
