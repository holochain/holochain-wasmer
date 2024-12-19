#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# static tests
cargo fmt --check
cargo clippy -- --deny warnings
( cd crates/guest && cargo clippy --target wasm32-unknown-unknown -- --deny warnings )

# tests the root workspace
cargo test --no-default-features --features error_as_host,wasmer_sys_dev ${1-} -- --nocapture

# test that test wasms build
cargo build --release -p test_wasm --target wasm32-unknown-unknown

# build wasm and run the "full" tests for wasmer_sys_dev
cargo test --release -p test --no-default-features --features wasmer_sys_dev ${1-} -- --nocapture
