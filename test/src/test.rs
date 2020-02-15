pub mod import;
pub mod load_wasm;

extern crate holochain_json_api;

use wasmer_runtime::Ctx;
use holochain_wasmer_host::guest;
use holochain_wasmer_host::*;

fn test_process_string(ctx: &mut Ctx, allocation_ptr: AllocationPtr) -> Result<AllocationPtr, Error> {
    let guest_bytes = guest::read_from_allocation_ptr(ctx, allocation_ptr)?;
    let processed_string = format!("host: {}", std::str::from_utf8(&guest_bytes)?);
    Ok(holochain_wasmer_host::bytes::to_allocation_ptr(processed_string.into_bytes()))
}

#[cfg(test)]
pub mod tests {

    use holochain_wasmer_host::guest;
    use crate::import::import_object;
    use crate::load_wasm::load_wasm;
    use wasmer_runtime::instantiate;
    use wasmer_runtime::Instance;
    use holochain_json_api::json::JsonString;
    use test_common::SomeStruct;
    use std::convert::TryInto;
    use holochain_wasmer_host::WasmResult;

    fn test_instance() -> Instance {
        instantiate(&load_wasm(), &import_object()).expect("build test instance")
    }

    #[test]
    fn stacked_test() {
        let result = guest::call(&mut test_instance(), "stacked_strings", vec![]).expect("stacked strings call");

        assert_eq!("first", result);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result_string = guest::call(&mut test_instance(), "native_type", JsonString::from(some_struct.clone()).to_bytes()).expect("native type handling");
        let wasm_result: WasmResult = JsonString::from_json(&result_string).try_into().expect("could not deserialize");

        match wasm_result {
            WasmResult::Ok(json_string) => {
                let result_struct: SomeStruct = json_string.try_into().unwrap();
                assert_eq!(
                    result_struct,
                    some_struct,
                );
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);

        let result_string = guest::call(&mut test_instance(), "process_string", starter_string.clone().into_bytes())
            .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(result_string, expected_string,);
    }
}
