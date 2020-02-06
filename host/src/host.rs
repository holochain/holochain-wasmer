mod allocate;
pub mod import;
pub mod load_wasm;

// Import the Filesystem so we can read our .wasm file
use crate::allocate::copy_string_to_guest;
use byte_slice_cast::AsSliceOf;
use common::allocate::string_allocation_ptr;
use common::allocate::string_from_allocation_ptr;
use common::error::Error;
use common::memory::Allocation;
use common::memory::AllocationPtr;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;
use std::convert::TryInto;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

fn read_guest_bytes(ctx: &Ctx, ptr: Ptr, len: Ptr) -> Vec<u8> {
    let memory = ctx.memory(0);
    let vec: Vec<u8> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    // Ok(std::str::from_utf8(&str_vec)?.into())
    vec
}

pub fn read_guest_bytes_from_allocation_ptr(
    ctx: &Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> Result<Vec<u8>, Error> {
    let view: MemoryView<u8> = ctx.memory(0).view();
    let bytes_vec: Vec<u8> = view
        [guest_allocation_ptr as _..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
        .iter()
        .map(|cell| cell.get())
        .collect();
    let guest_allocation: Allocation = bytes_vec
        .as_slice_of::<u64>()?
        .try_into()
        .expect("wrong number of array elements");

    Ok(read_guest_bytes(
        ctx,
        guest_allocation[0],
        guest_allocation[1],
    ))
}

fn host_process_string(ctx: &mut Ctx, ptr: i64, cap: i64) -> Result<AllocationPtr, Error> {
    let guest_bytes = read_guest_bytes(ctx, ptr.try_into()?, cap.try_into()?);
    let processed_string = format!("host: {}", std::str::from_utf8(&guest_bytes)?);
    Ok(string_allocation_ptr(processed_string))
}

fn host_copy_string(ctx: &mut Ctx, host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr) {
    let s = string_from_allocation_ptr(host_allocation_ptr);
    copy_string_to_guest(ctx, guest_string_ptr, s);
}

pub fn guest_call(instance: &Instance, call: &str, payload: &String) -> Result<String, Error> {
    let starter_string_allocation_ptr = string_allocation_ptr(payload.clone());

    let guest_allocation_ptr = match instance
        .call(
            call,
            &[Value::I64(starter_string_allocation_ptr.try_into()?)],
        )
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    Ok(std::str::from_utf8(&read_guest_bytes_from_allocation_ptr(
        &instance.context(),
        guest_allocation_ptr,
    )?)?
    .into())
}

#[cfg(test)]
pub mod tests {

    use crate::guest_call;
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
