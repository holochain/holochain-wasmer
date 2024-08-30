#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

cargo clippy --no-default-features --features wasmer_wamr
( cd test && cargo clippy --no-default-features --features wasmer_wamr )

# build wasm and run the "full" tests for wasmer_wamr
cargo test --release --manifest-path test/Cargo.toml --no-default-features --features wasmer_wamr ${1-} -- --nocapture
