#! /usr/bin/env bash
cargo test

cargo test-fuzz "$FUZZ_TARGET" --no-default-features --features wasmer_sys_prod
