pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust.
pub mod plru;
pub mod prelude;

// At least one wasmer backend must be enabled. The two backends (`wasmer_sys`
// and `wasmer_wasmi`) are independent and can be enabled simultaneously; the
// caller picks one at runtime by passing the appropriate engine factory to
// [`module::ModuleBuilder::new`].
#[cfg(not(any(feature = "wasmer_sys", feature = "wasmer_wasmi")))]
compile_error!(
    "at least one wasmer backend feature must be enabled: `wasmer_sys` (with `wasmer_sys_cranelift` and/or `wasmer_sys_llvm`) and/or `wasmer_wasmi`"
);

// `wasmer_sys` requires at least one compiler sub-feature.
#[cfg(all(
    feature = "wasmer_sys",
    not(any(feature = "wasmer_sys_cranelift", feature = "wasmer_sys_llvm"))
))]
compile_error!(
    "the `wasmer_sys` feature requires at least one of `wasmer_sys_cranelift` or `wasmer_sys_llvm` to also be enabled"
);
