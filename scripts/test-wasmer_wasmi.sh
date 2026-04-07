#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1

# Tests run with --release because wasmer's wasmi backend trips Rust's
# stabilised debug-mode unsafe-precondition checks. Tracked upstream as
# https://github.com/wasmerio/wasmer/issues/6392; drop the --release flags
# once that is fixed.

# tests the root workspace
cargo test --release --no-default-features --features wasmer_wasmi ${1-} -- --nocapture

# tests the root workspace, error_as_host
cargo test --release --no-default-features --features error_as_host,wasmer_wasmi ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer_wasmi
cargo test --release -p tests --no-default-features --features wasmer_wasmi ${1-} -- --nocapture
