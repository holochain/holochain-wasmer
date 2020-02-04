use crate::allocate::copy_allocation_to_guest;
use crate::host_copy_string;
use crate::host_process_string;
use wasmer_runtime::func;
use wasmer_runtime::imports;
use wasmer_runtime::memory::MemoryView;
use wasmer_runtime::Ctx;
use wasmer_runtime::ImportObject;

fn prn(ctx: &mut Ctx, u: u64) {
    let view: MemoryView<u8> = ctx.memory(0).view();
    println!("prn: {}", u);
    println!("zz: {}", view.len());
}

pub fn import_object() -> ImportObject {
    imports! {
        "env" => {
            "__host_process_string" => func!(host_process_string),
            "__copy_allocation_to_guest" => func!(copy_allocation_to_guest),
            "__host_copy_string" => func!(host_copy_string),
            "__prn" => func!(prn),
        },
    }
}
