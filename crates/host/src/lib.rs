pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust.
pub mod plru;
pub mod prelude;

#[cfg(all(feature = "wasmer_sys", feature = "wasmer_wasmi"))]
compile_error!(
    "features \"wasmer_sys\" and \"wasmer_wasmi\" are mutually exclusive — pick exactly one"
);

#[cfg(all(not(feature = "wasmer_sys"), not(feature = "wasmer_wasmi")))]
compile_error!("One of: `wasmer_sys`, `wasmer_wasmi` features must be enabled. Please, pick one.");
