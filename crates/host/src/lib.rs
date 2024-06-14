#[cfg(all(feature = "wasmer_sys", feature = "wasmer_wamr"))]
compile_error!("feature \"wasmer_sys\" and feature \"wasmer_wamr\" cannot be enabled at the same time");