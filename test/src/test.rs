pub mod import;
pub mod load_wasm;

use wasmer_runtime::Ctx;
use common::memory::AllocationPtr;
use common::error::Error;
use std::convert::TryInto;
use host::read_guest_bytes;
use common::allocate::string_allocation_ptr;

fn test_process_string(ctx: &mut Ctx, ptr: i64, cap: i64) -> Result<AllocationPtr, Error> {
    let guest_bytes = read_guest_bytes(ctx, ptr.try_into()?, cap.try_into()?);
    let processed_string = format!("host: {}", std::str::from_utf8(&guest_bytes)?);
    Ok(string_allocation_ptr(processed_string))
}

#[cfg(test)]
pub mod tests {

    use host::guest_call;
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

        let result_string = guest_call(&test_instance(), "process_string", &starter_string)
            .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(result_string, expected_string,);
    }
}
