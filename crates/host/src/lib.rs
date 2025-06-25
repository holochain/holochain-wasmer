pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust.
pub mod plru;
pub mod prelude;

#[cfg(all(feature = "wasmer_sys", feature = "wasmer_wamr"))]
compile_error!(
    "feature \"wasmer_sys\" and feature \"wasmer_wamr\" cannot be enabled at the same time"
);

#[cfg(all(feature = "wasmer_sys", feature = "wasmer_v8"))]
compile_error!(
    "feature \"wasmer_sys\" and feature \"wasmer_v8\" cannot be enabled at the same time"
);

#[cfg(all(feature = "wasmer_wamr", feature = "wasmer_v8"))]
compile_error!(
    "feature \"wasmer_wamr\" and feature \"wasmer_v8\" cannot be enabled at the same time"
);

#[cfg(all(
    not(feature = "wasmer_sys"),
    not(feature = "wasmer_wamr"),
    not(feature = "wasmer_v8")
))]
compile_error!(
    "One of: `wasmer_sys`, `wasmer_wamr`, `wasmer_v8` features must be enabled. Please, pick one."
);
