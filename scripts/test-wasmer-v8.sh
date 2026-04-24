#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# tests the root workspace
cargo test --no-default-features --features wasmer-v8 ${1-} -- --nocapture

# tests the root workspace, error-as-host
cargo test --no-default-features --features error-as-host,wasmer-v8 ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer-v8
cargo test -p tests --no-default-features --features wasmer-v8 ${1-} -- --nocapture
