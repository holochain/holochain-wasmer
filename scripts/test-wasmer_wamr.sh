#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1


# tests the root workspace
cargo test --no-default-features --features error_as_host,wasmer_wamr ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer_wamr
cargo test --release -p test --no-default-features --features wasmer_wamr ${1-} -- --nocapture
