pub mod import;
pub mod load_wasm;

extern crate holochain_json_api;

use wasmer_runtime::Ctx;
use holochain_wasmer_host::guest;
use holochain_wasmer_host::*;

fn test_process_string(ctx: &mut Ctx, allocation_ptr: AllocationPtr) -> Result<AllocationPtr, WasmError> {
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
    use test_common::SomeStruct;

    fn test_instance() -> Instance {
        instantiate(&load_wasm(), &import_object()).expect("build test instance")
    }

    #[test]
    fn stacked_test() {
        let result_bytes = guest::call_bytes(&mut test_instance(), "stacked_strings", vec![]).expect("stacked strings call");
        let result_str = std::str::from_utf8(&result_bytes).unwrap();

        assert_eq!("first", result_str);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct = guest::call(&mut test_instance(), "native_type", some_struct.clone()).expect("native type handling");

        assert_eq!(
            some_struct,
            result,
        );
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);

        let result_bytes = guest::call_bytes(&mut test_instance(), "process_string", starter_string.clone().into_bytes())
            .expect("process string call");
        let result_str = std::str::from_utf8(&result_bytes).unwrap();

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(result_str, &expected_string,);
    }
}
