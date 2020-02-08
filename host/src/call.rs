use common::bytes;
use common::error::Error;
use std::convert::TryInto;
use wasmer_runtime::Instance;
use wasmer_runtime::Value;

pub fn guest_call(instance: &mut Instance, call: &str, payload: Vec<u8>) -> Result<String, Error> {
    let host_allocation_ptr = bytes::to_allocation_ptr(payload);

    // this requires that the guest exported function being called knows what to do with a
    // host allocation pointer
    let guest_allocation_ptr = match instance
        .call(call, &[Value::I64(host_allocation_ptr.try_into()?)])
        .expect("call error")[0]
    {
        Value::I64(i) => i as u64,
        _ => unreachable!(),
    };

    Ok(
        std::str::from_utf8(&crate::bytes::read_from_guest_using_allocation_ptr(
            instance.context_mut(),
            guest_allocation_ptr,
        )?)?
        .into(),
    )
}
