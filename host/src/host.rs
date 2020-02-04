mod allocate;
pub mod import;
pub mod load_wasm;

// Import the Filesystem so we can read our .wasm file
use crate::allocate::copy_string_to_guest;
use common::allocate::string_allocation_ptr;
use common::allocate::string_from_allocation_ptr;
use common::memory::Allocation;
use common::memory::AllocationPtr;
use common::memory::Ptr;
use common::memory::ALLOCATION_BYTES_ITEMS;
use std::convert::TryInto;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;
use byte_slice_cast::AsSliceOf;

pub fn write_guest_string(instance: &mut Instance, s: String) -> Allocation {
    let guest_ptr = match instance
        .call("allocate", &[Value::I64(s.len() as _)])
        .expect("run pre alloc")[0]
    {
        Value::I64(i) => i as Ptr,
        _ => unreachable!(),
    };

    let guest_allocation = [guest_ptr as Ptr, s.len() as Ptr];
    copy_string_to_guest(instance.context_mut(), guest_allocation[0], s);
    guest_allocation
}

fn read_guest_string(ctx: &Ctx, ptr: Ptr, len: Ptr) -> String {
    println!("rgs {} {}", ptr, len);
    let memory = ctx.memory(0);
    let str_vec: Vec<_> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    std::str::from_utf8(&str_vec).unwrap().into()
}

pub fn read_guest_string_from_allocation_ptr(
    ctx: &Ctx,
    guest_allocation_ptr: AllocationPtr,
) -> String {
    let view: MemoryView<u8> = ctx.memory(0).view();
    let bytes_vec: Vec<u8> = view
        [guest_allocation_ptr as _..(guest_allocation_ptr + ALLOCATION_BYTES_ITEMS as Ptr) as _]
        .iter()
        .map(|cell| cell.get())
        .collect();
    let guest_allocation: Allocation = bytes_vec.as_slice_of::<u64>().unwrap().try_into().expect("wrong number of array elements");

    println!("xx {}", guest_allocation_ptr);
    println!("yy {}", view.len());

    read_guest_string(ctx, guest_allocation[0], guest_allocation[1])
}

fn host_process_string(ctx: &mut Ctx, ptr: i64, cap: i64) -> AllocationPtr {
    let guest_string = read_guest_string(ctx, ptr.try_into().unwrap(), cap.try_into().unwrap());
    let processed_string = format!("host: {}", guest_string);
    string_allocation_ptr(processed_string)
}

fn host_copy_string(ctx: &mut Ctx, host_allocation_ptr: AllocationPtr, guest_string_ptr: Ptr) {
    let s = string_from_allocation_ptr(host_allocation_ptr);
    copy_string_to_guest(ctx, guest_string_ptr, s);
}

#[cfg(test)]
pub mod tests {

    use crate::import::import_object;
    use crate::load_wasm::load_wasm;
    use crate::read_guest_string_from_allocation_ptr;
    use common::allocate::string_allocation_ptr;
    use std::convert::TryInto;
    use wasmer_runtime::instantiate;
    use wasmer_runtime::Value;

    #[test]
    fn do_it() {
        let instance = instantiate(&load_wasm(), &import_object()).expect("build instance");
        let starter_string = String::from("foobar");
        // let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((U16_MAX * 1) as usize);
        let _ = "foo".repeat(std::u16::MAX as _);

        // let [guest_ptr, guest_len] = write_guest_string(&mut instance, starter_string.clone());
        // println!("{} {}", guest_ptr, guest_len);

        let starter_string_allocation_ptr = string_allocation_ptr(starter_string);

        println!("ssap {}", &starter_string_allocation_ptr);

        let guest_allocation_ptr = match instance
            .call(
                "process_string",
                &[Value::I64(
                    starter_string_allocation_ptr.try_into().unwrap(),
                )],
            )
            .expect("call error xx")[0]
        {
            Value::I64(i) => i as u64,
            _ => unreachable!(),
        };
        println!("gap {}", guest_allocation_ptr);

        let result_string =
            read_guest_string_from_allocation_ptr(&instance.context(), guest_allocation_ptr);
        println!("result {}", result_string);
    }
}
