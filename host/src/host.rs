// Import the Filesystem so we can read our .wasm file
use common::bits_n_pieces::u64_merge_bits;
use std::fs::File;
use std::io::prelude::*;
use wasmer_runtime::Ctx;
use wasmer_runtime::Value;
use wasmer_runtime::Instance;

// Import the wasmer runtime so we can use it
use wasmer_runtime::{func, imports, ImportObject};

// Create an absolute path to the Wasm file
const WASM_FILE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/wasm32-unknown-unknown/release/guest.wasm"
);

pub fn load_wasm() -> Vec<u8> {
    // Let's read in our .wasm file as bytes

    // Let's open the file.
    let mut file = File::open(WASM_FILE_PATH).expect(&format!("wasm file at {}", WASM_FILE_PATH));

    // Let's read the file into a Vec
    let mut wasm_vec = Vec::new();
    file.read_to_end(&mut wasm_vec)
        .expect("Error reading the wasm file");
    wasm_vec
}

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__host_process_string" => func!(host_process_string),
        },
    }
}

pub fn write_guest_string(instance: &mut Instance, s: String) -> (i32, i32) {
    let guest_ptr = match instance
        .call("pre_alloc_string", &[Value::I32(s.len() as _)])
        .expect("run pre alloc")[0]
    {
        Value::I32(i) => i,
        _ => unreachable!(),
    };

    let memory = instance.context_mut().memory(0);

    for (byte, cell) in s
        .bytes()
        .zip(memory.view()[guest_ptr as _..(guest_ptr + s.len() as i32) as _].iter()) {
            cell.set(byte)
    }

    (guest_ptr, s.len() as i32)
}

fn read_guest_string(ctx: &Ctx, ptr: i32, len: i32) -> String {
    let memory = ctx.memory(0);
    let str_vec: Vec<_> = memory.view()[ptr as usize..(ptr + len) as usize]
        .iter()
        .map(|cell| cell.get())
        .collect();

    // Convert the subslice to a `&str`.
    unsafe { std::str::from_utf8_unchecked(&str_vec) }.into()
}

fn host_process_string(ctx: &mut Ctx, ptr: i32, cap: i32) -> u64 {
    println!("hiii {} {}", ptr, cap);
    let guest_string = read_guest_string(ctx, ptr, cap);
    println!("guest_string {}", guest_string);
    let processed_string = format!("host: {}", guest_string);
    u64_merge_bits(processed_string.as_ptr() as _, processed_string.len() as _)
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use wasmer_runtime::instantiate;
    use common::bits_n_pieces::u64_split_bits;
    use std::convert::TryInto;

    #[test]
    fn do_it() {
        let mut instance = instantiate(&load_wasm(), &import_object()).expect("build instance");
        let starter_string = String::from("foobar");

        let (guest_ptr, guest_len) = write_guest_string(&mut instance, starter_string.clone());
        println!("{} {}", guest_ptr, guest_len);

        let (result_ptr, result_len) = u64_split_bits(match instance.call("process_string", &[Value::I32(guest_ptr), Value::I32(guest_len)]).expect("call error")[0] {
            Value::I64(i) => i as u64,
            _ => unreachable!(),
        });
        println!("{} {}", result_ptr, result_len);

        let result_string = read_guest_string(&instance.context(), result_ptr.try_into().unwrap(), result_len.try_into().unwrap());
        println!("{}", result_string);
    }
}
