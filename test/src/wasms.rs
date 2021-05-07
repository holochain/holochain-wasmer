pub const EMPTY: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_empty.wasm"
));

pub const IO: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_io.wasm"
));

pub const TEST: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm.wasm"
));

pub const MEMORY: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_memory.wasm"
));
