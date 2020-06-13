pub mod import;
pub mod load_wasm;

extern crate holochain_serialized_bytes;

use holochain_wasmer_host::import::set_context_data;
use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;
use test_common::StringType;

fn test_process_string(ctx: &mut Ctx, guest_ptr: GuestPtr) -> Result<Len, WasmError> {
    let processed_string: StringType = guest::from_guest_ptr(ctx, guest_ptr)?;
    let processed_string = format!("host: {}", String::from(processed_string));
    let sb: SerializedBytes = StringType::from(processed_string).try_into()?;
    Ok(set_context_data(ctx, sb))
}

fn test_process_struct(ctx: &mut Ctx, guest_ptr: GuestPtr) -> Result<Len, WasmError> {
    let mut some_struct: SomeStruct = guest::from_guest_ptr(ctx, guest_ptr)?;
    some_struct.process();
    let sb: SerializedBytes = some_struct.try_into()?;
    Ok(set_context_data(ctx, sb))
}

fn debug(_ctx: &mut Ctx, some_number: WasmSize) -> Result<Len, WasmError> {
    println!("debug {:?}", some_number);
    Ok(0)
}

#[cfg(test)]
pub mod tests {

    use crate::import::import_object;
    use crate::load_wasm::load_wasm;
    use holochain_wasmer_host::prelude::*;
    use test_common::SomeStruct;
    use test_common::StringType;

    #[test]
    fn smoke_module() {
        let wasm = load_wasm();
        let module: Module = module(&wasm, &wasm).unwrap();
        assert!(module
            .info()
            .exports
            .contains_key("__allocation_ptr_uninitialized"));
    }

    fn test_instance() -> Instance {
        let wasm = load_wasm();
        instantiate(&wasm, &wasm, &import_object()).expect("build test instance")
    }

    #[test]
    fn stacked_test() {
        let result: StringType =
            guest::call(&mut test_instance(), "stacked_strings", ()).expect("stacked strings call");

        assert_eq!("first", &String::from(result));
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);

        let result: StringType = guest::call(
            &mut test_instance(),
            "process_string",
            StringType::from(starter_string.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(&String::from(result), &expected_string,);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct =
            guest::call(&mut test_instance(), "native_type", some_struct.clone())
                .expect("native type handling");

        assert_eq!(some_struct, result,);
    }

    #[test]
    fn native_struct_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct =
            guest::call(&mut test_instance(), "process_native", some_struct.clone()).unwrap();

        let expected = SomeStruct::new(format!("processed: {}", some_inner));
        assert_eq!(result, expected,);
    }

    #[test]
    fn ret_test() {
        let result: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "some_ret", ());
        match result {
            Ok(some_struct) => {
                assert_eq!(SomeStruct::new("foo".into()), some_struct,);
            }
            Err(_) => unreachable!(),
        };

        let err: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "some_ret_err", ());
        match err {
            Err(wasm_error) => assert_eq!(WasmError::Zome("oh no!".into()), wasm_error,),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_result_test() {
        let success_result: Result<SomeStruct, WasmError> =
            guest::call(&mut test_instance(), "try_result_succeeds", ());
        match success_result {
            Ok(some_struct) => {
                assert_eq!(SomeStruct::new("foo".into()), some_struct,);
            }
            Err(_) => unreachable!(),
        };

        let fail_result: Result<(), WasmError> =
            guest::call(&mut test_instance(), "try_result_fails_fast", ());
        match fail_result {
            Err(wasm_error) => {
                assert_eq!(WasmError::Zome("it fails!".into()), wasm_error,);
            }
            Ok(_) => unreachable!(),
        };
    }
}
