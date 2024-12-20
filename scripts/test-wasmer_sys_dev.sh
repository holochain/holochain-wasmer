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
cargo test --no-default-features --features wasmer_sys_dev ${1-} -- --nocapture

# tests the root workspace, error_as_host
cargo test --no-default-features --features error_as_host,wasmer_sys_dev ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer_sys_dev
cargo test --release -p tests --no-default-features --features wasmer_sys_dev ${1-} -- --nocapture
