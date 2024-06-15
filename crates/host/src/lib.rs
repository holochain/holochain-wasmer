#[cfg(all(feature = "wasmer_sys", feature = "wasmer_wamr"))]
compile_error!("feature \"wasmer_sys\" and feature \"wasmer_wamr\" cannot be enabled at the same time");

#[cfg(all(
  not(feature = "wasmer_sys"),
  not(feature = "wasmer_wamr"),
))]
compile_error!(
  "One of: `wasmer_sys`, `wasmer_wamr` features must be enabled. Please, pick one."
);