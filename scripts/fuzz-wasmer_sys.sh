#! /usr/bin/env bash
cargo test
cargo test --manifest-path test/Cargo.toml

cargo test-fuzz "$FUZZ_TARGET" --no-default-features --features wasmer_sys_dev
