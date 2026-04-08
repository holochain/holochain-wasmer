pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust.
pub mod plru;
pub mod prelude;

// At least one wasmer backend must be enabled. The two backends (`wasmer-sys`
// and `wasmer-wasmi`) are independent and can be enabled simultaneously; the
// caller picks one at runtime by passing the appropriate engine factory to
// [`module::ModuleBuilder::new`].
#[cfg(not(any(feature = "wasmer-sys", feature = "wasmer-wasmi")))]
compile_error!(
    "at least one wasmer backend feature must be enabled: `wasmer-sys` (with `wasmer-sys-cranelift` and/or `wasmer-sys-llvm`) and/or `wasmer-wasmi`"
);

// `wasmer-sys` requires at least one compiler sub-feature.
#[cfg(all(
    feature = "wasmer-sys",
    not(any(feature = "wasmer-sys-cranelift", feature = "wasmer-sys-llvm"))
))]
compile_error!(
    "the `wasmer-sys` feature requires at least one of `wasmer-sys-cranelift` or `wasmer-sys-llvm` to also be enabled"
);
