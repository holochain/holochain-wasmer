#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# tests the root workspace that doesn't include any wasm code
cargo test --no-default-features --features error_as_host,wasmer_sys_prod ${1-} -- --nocapture

# test that everything builds
cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown

# build wasm and run the "full" tests for wasmer_sys_prod
cargo test --release --manifest-path test/Cargo.toml --no-default-features --features wasmer_sys_prod ${1-} -- --nocapture
