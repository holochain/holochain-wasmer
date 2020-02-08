pub mod import;
pub mod load_wasm;

use wasmer_runtime::Ctx;
use common::AllocationPtr;
use common::error::Error;
use std::convert::TryInto;
use host::bytes;

fn test_process_string(ctx: &mut Ctx, ptr: i64, cap: i64) -> Result<AllocationPtr, Error> {
    let guest_bytes = bytes::read_from_guest(ctx, ptr.try_into()?, cap.try_into()?);
    let processed_string = format!("host: {}", std::str::from_utf8(&guest_bytes)?);
    Ok(common::bytes::to_allocation_ptr(processed_string.into_bytes()))
}

#[cfg(test)]
pub mod tests {

    use host::call;
    use crate::import::import_object;
    use crate::load_wasm::load_wasm;
    use wasmer_runtime::instantiate;
    use wasmer_runtime::Instance;

    fn test_instance() -> Instance {
        instantiate(&load_wasm(), &import_object()).expect("build test instance")
    }

    #[test]
    fn do_it() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);

        let result_string = call::guest_call(&mut test_instance(), "process_string", starter_string.clone().into_bytes())
            .expect("process string call");

        // let expected_string = format!("host: guest: {}", &starter_string);
        let expected_string = String::from("foo");

        assert_eq!(result_string, expected_string,);
    }
}
