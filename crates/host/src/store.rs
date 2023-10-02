// use once_cell::sync::Lazy;
// use std::sync::Arc;
// use wasmer::wasmparser::Operator;
// use wasmer::CompilerConfig;
// use wasmer::Cranelift;
// use wasmer::Store;
// use wasmer_middlewares::Metering;

// const METERING_LIMIT: u64 = 10_000_000_000;

// pub static STORE: Lazy<Store> = Lazy::new(|| {
//     let const_function = |_operator: &Operator| -> u64 { 1 };
//     let metering = Metering::new(METERING_LIMIT, const_function);
//     let mut cranelift = Cranelift::default();
//     cranelift
//         .canonicalize_nans(true)
//         .push_middleware(Arc::new(metering));
//     Store::new(cranelift)
// });
