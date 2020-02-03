pub mod import;
pub mod load_wasm;
mod allocate;

// Import the Filesystem so we can read our .wasm file
use wasmer_runtime::Ctx;
use wasmer_runtime::Value;
use wasmer_runtime::Instance;
use common::memory::Ptr;
use common::memory::Allocation;
use std::slice;
use crate::allocate::copy_string_to_guest;
use std::convert::TryInto;
use common::memory::Len;
use common::memory::AllocationPtr;
use common::allocate::string_allocation_ptr;

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

pub fn read_guest_string_from_allocation_ptr(ctx: &Ctx, guest_allocation_ptr: AllocationPtr) -> String {
    let view = ctx.memory(0).view();

    let guest_string_ptr: Ptr = view[guest_allocation_ptr as usize].get();
    let guest_string_len: Len = view[(guest_allocation_ptr + 1) as usize].get();

    read_guest_string(ctx, guest_string_ptr, guest_string_len)
}

fn host_process_string(ctx: &mut Ctx, ptr: i64, cap: i64) -> u64 {
    let guest_string = read_guest_string(ctx, ptr.try_into().unwrap(), cap.try_into().unwrap());
    let processed_string = format!("host: {}", guest_string);
    string_allocation_ptr(processed_string)
}

fn host_copy_string(ctx: &mut Ctx, host_ptr: Ptr, guest_ptr: Ptr, len: Len) {
    let slice = unsafe { slice::from_raw_parts(host_ptr as _, len as _) };
    let s = String::from(std::str::from_utf8(slice).unwrap());
    copy_string_to_guest(ctx, guest_ptr, s);
}

#[cfg(test)]
pub mod tests {

    use crate::load_wasm::load_wasm;
    use crate::import::import_object;
    use crate::write_guest_string;
    // use crate::read_guest_string;
    use wasmer_runtime::Value;
    use wasmer_runtime::instantiate;
    use std::convert::TryInto;
    use crate::read_guest_string_from_allocation_ptr;

    #[test]
    fn do_it() {
        let mut instance = instantiate(&load_wasm(), &import_object()).expect("build instance");
        let starter_string = String::from("foobar");
        // let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((U16_MAX * 1) as usize);
        let _ = "foo".repeat(std::u16::MAX as _);

        let [guest_ptr, guest_len] = write_guest_string(&mut instance, starter_string.clone());
        println!("{} {}", guest_ptr, guest_len);

        let guest_allocation_ptr = match instance.call("process_string", &[Value::I64(guest_ptr.try_into().unwrap()), Value::I64(guest_len.try_into().unwrap())]).expect("call error xx")[0] {
            Value::I64(i) => i as u64,
            _ => unreachable!(),
        };
        println!("{}", guest_allocation_ptr);

        let result_string = read_guest_string_from_allocation_ptr(&instance.context(), guest_allocation_ptr);
        println!("result {}", result_string);
    }
}
