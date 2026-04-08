#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# static tests
cargo fmt --check
cargo clippy -- --deny warnings
( cd crates/guest && cargo clippy --target wasm32-unknown-unknown -- --deny warnings )

# test that test wasms build
cargo build --release -p test_wasm_core --target wasm32-unknown-unknown

# tests the root workspace
cargo test --no-default-features --features wasmer-sys-cranelift ${1-} -- --nocapture

# tests the root workspace, error-as-host
cargo test --no-default-features --features error-as-host,wasmer-sys-cranelift ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer-sys-cranelift
cargo test --release -p tests --no-default-features --features wasmer-sys-cranelift ${1-} -- --nocapture
