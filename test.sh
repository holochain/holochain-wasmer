#!/usr/bin/env bash
set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1
cargo fmt
( cd test && cargo fmt )
( cd crates/guest && cargo fmt )

cargo clippy
( cd test && cargo clippy )
( cd crates/guest && cargo clippy --target wasm32-unknown-unknown )

# tests the root workspace that doesn't include any wasm code
cargo test ${1-} -- --nocapture

# test that everything builds
cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown

# build wasm and run the "full" tests for wasmer_sys
cargo test --release --manifest-path test/Cargo.toml ${1-} -- --nocapture 

cargo clippy --no-default-features --features wasmer_wamr
( cd test && cargo clippy --no-default-features --features wasmer_wamr )
( cd crates/guest && cargo clippy --target wasm32-unknown-unknown --no-default-features --features wasmer_wamr )

# build wasm and run the "full" tests for wasmer_wamr
cargo test --release --manifest-path test/Cargo.toml --no-default-features --features wasmer_wamr ${1-} -- --nocapture
