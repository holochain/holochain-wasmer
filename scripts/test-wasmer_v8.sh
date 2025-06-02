#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1


# tests the root workspace
cargo test --no-default-features --features wasmer_v8  ${1-} -- --nocapture

# tests the root workspace, error_as_host
cargo test --no-default-features --features error_as_host,wasmer_v8 ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer_v8
cargo test --release -p tests --no-default-features --features wasmer_v8 ${1-} -- --nocapture
