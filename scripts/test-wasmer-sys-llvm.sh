#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# tests the root workspace
cargo test --no-default-features --features wasmer-sys-llvm ${1-} -- --nocapture

# tests the root workspace, error-as-host
cargo test --no-default-features --features error-as-host,wasmer-sys-llvm ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer-sys-llvm
cargo test --release -p tests --no-default-features --features wasmer-sys-llvm ${1-} -- --nocapture
