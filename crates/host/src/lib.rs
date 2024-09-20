pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust.
pub mod plru;
pub mod prelude;

#[cfg(all(wasmer_sys, wasmer_wamr))]
compile_error!("cfg \"wasmer_sys\" and feature \"wasmer_wamr\" cannot be enabled at the same time");

#[cfg(all(not(wasmer_sys), not(wasmer_wamr)))]
compile_error!("One of: `wasmer_sys`, `wasmer_wamr` cfg must be enabled. Please, pick one.");

// Temporarily include a fork of wasmer from the git branch 'wamr', until it is officially released in wasmer v5
#[cfg(wasmer_wamr)]
extern crate hc_wasmer as wasmer;
