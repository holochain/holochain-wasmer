pub mod import;
pub mod wasms;

use holochain_wasmer_host::env::Env;
use holochain_wasmer_host::import::set_host_return_encoded;
use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;

fn test_process_string(env: &Env, guest_ptr: GuestPtr) -> Result<Len, WasmError> {
    dbg!(&guest_ptr);
    dbg!(&env.memory_ref());
    let maybe_memory = env.memory_ref();
    // dbg!(&env.host_return_encoded);
    dbg!(&maybe_memory);

    let string: String =
        guest::from_guest_ptr(env.memory_ref().ok_or(WasmError::Memory)?, guest_ptr)?;
    let processed_string = format!("host: {}", string);
    Ok(set_host_return_encoded(
        env,
        Ok::<String, WasmError>(processed_string),
    )?)
}

fn test_process_struct(env: &Env, guest_ptr: GuestPtr) -> Result<Len, WasmError> {
    // dbg!(env.memory_ref());
    // let maybe_memory = env.memory.read();
    let mut some_struct: SomeStruct =
        guest::from_guest_ptr(env.memory_ref().ok_or(WasmError::Memory)?, guest_ptr)?;
    some_struct.process();
    Ok(set_host_return_encoded(
        env,
        Ok::<SomeStruct, WasmError>(some_struct),
    )?)
}

fn debug(_env: &Env, some_number: WasmSize) -> Result<Len, WasmError> {
    println!("debug {:?}", some_number);
    Ok(0)
}

fn pages(env: &Env, _: WasmSize) -> Result<WasmSize, WasmError> {
    // let maybe_memory = env.memory.read();
    Ok(env.memory_ref().ok_or(WasmError::Memory)?.size().0)
}

#[cfg(test)]
pub mod tests {

    use crate::import::import_object;
    use crate::wasms;
    use holochain_wasmer_host::prelude::*;
    use test_common::SomeStruct;
    use test_common::StringType;

    fn test_instance(wasm: &[u8]) -> Instance {
        let engine = JIT::new(Singlepass::new()).engine();
        let store = Store::new(&engine);
        let env = Env::new();
        let module = Module::new(&store, wasm).unwrap();
        dbg!("z");
        let instance = Instance::new(&module, &import_object(&store, &env)).unwrap();
        dbg!("x");
        instance
    }

    #[test]
    fn bytes_round_trip() {
        let mut instance = test_instance(wasms::MEMORY);

        let _: () = guest::call(&mut instance, "bytes_round_trip", ()).unwrap();
    }

    // #[test]
    // fn smoke_module() {
    //     let wasm = wasms::TEST;
    //     let module: Module = module::<String>(&wasm, &wasm, None).unwrap();
    //     assert!(module.info().exports.contains_key("__hcallocate"));
    // }

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
