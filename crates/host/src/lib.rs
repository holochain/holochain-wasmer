//! Host-side runtime for executing wasm guests via [wasmer]. Used by
//! Holochain ribosomes to instantiate, cache and call zome wasms, and
//! by anything else that needs to embed wasm with a similar shape: a
//! long-lived host process making short-lived calls into many guest
//! modules built from cached compiled artifacts.
//!
//! The two entry points most callers reach for are
//! [`module::ModuleCache`] for managing compiled-module reuse and
//! [`guest::call`] for invoking a guest function with a serializable
//! payload. Errors from both sides flow through [`prelude::WasmError`].
//!
//! # Cargo features
//!
//! At least one wasmer backend must be enabled. The two backends are
//! independent and can be enabled simultaneously; the choice is then
//! made at the call site by passing the appropriate engine factory to
//! [`module::ModuleBuilder::new`].
//!
//! - **`wasmer-sys`** *(default)* — enables the wasmer native backend
//!   that compiles wasm via a real compiler. Requires at least one of
//!   the compiler sub-features below.
//!   - **`wasmer-sys-cranelift`** *(default)* — Cranelift compiler.
//!     Fast to compile, good runtime performance, the default
//!     development compiler.
//!   - **`wasmer-sys-llvm`** — LLVM compiler. Slower to compile, faster
//!     at runtime, the recommended choice for production deployments
//!     where compile time is amortised over many calls. Can be enabled
//!     alongside `wasmer-sys-cranelift`.
//! - **`wasmer-v8`** — enables the V8-backed wasmer backend. V8
//!   itself is a JIT, but it can be initialized with `--jitless`
//!   (see [`module::v8::set_flags_from_string`]) which forces an
//!   interpreter-only execution mode. That combination is what
//!   makes this backend usable on iOS, where Apple does not grant
//!   the JIT entitlement to non-browser apps.
//! - **`error-as-host`** *(default)* — when constructing a
//!   [`prelude::WasmError`] from a bare `String`, classify it as
//!   [`prelude::WasmErrorInner::Host`] rather than
//!   [`prelude::WasmErrorInner::Guest`]. Hosts that build error
//!   strings should enable this; guests should leave it off.
//! - **`debug-memory`** — enable verbose `tracing::debug!` logging for
//!   every host↔guest memory copy. Off by default; useful only when
//!   chasing memory bugs.
//!
//! [wasmer]: https://docs.rs/wasmer

pub mod env;
pub mod error;
pub mod guest;
pub mod module;
/// Adapted from: https://raw.githubusercontent.com/ticki/plru/master/src/lib.rs
/// Updated for latest stable rust. Vendored largely as-is, so several
/// of the cache size aliases and `Cache` methods are unused internally
/// — `#[allow(dead_code)]` keeps the upstream shape intact rather than
/// carving the file up.
#[allow(dead_code)]
pub(crate) mod plru;
pub mod prelude;

// At least one wasmer backend must be enabled. The two backends (`wasmer-sys`
// and `wasmer-v8`) are independent and can be enabled simultaneously; the
// caller picks one at runtime by passing the appropriate engine factory to
// [`module::ModuleBuilder::new`].
#[cfg(not(any(feature = "wasmer-sys", feature = "wasmer-v8")))]
compile_error!(
    "at least one wasmer backend feature must be enabled: `wasmer-sys` (with `wasmer-sys-cranelift` and/or `wasmer-sys-llvm`) and/or `wasmer-v8`"
);

// `wasmer-sys` requires at least one compiler sub-feature.
#[cfg(all(
    feature = "wasmer-sys",
    not(any(feature = "wasmer-sys-cranelift", feature = "wasmer-sys-llvm"))
))]
compile_error!(
    "the `wasmer-sys` feature requires at least one of `wasmer-sys-cranelift` or `wasmer-sys-llvm` to also be enabled"
);
