#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1


# NOTE: the wamr backend tests are run in --release mode because wasmer's wamr
# backend has a UB bug that trips Rust's debug-mode unsafe-precondition checks
# when calling wasm functions with no return values: slice::from_raw_parts is
# invoked with a possibly-null pointer when results.size == 0. Tracked upstream
# as https://github.com/wasmerio/wasmer/issues/6392. Once that is fixed we can
# drop the --release flags below.

# tests the root workspace
cargo test --release --no-default-features --features wasmer_wamr ${1-} -- --nocapture

# tests the root workspace, error_as_host
cargo test --release --no-default-features --features error_as_host,wasmer_wamr ${1-} -- --nocapture

# build wasm and run the "full" tests for wasmer_wamr
cargo test --release -p tests --no-default-features --features wasmer_wamr ${1-} -- --nocapture
