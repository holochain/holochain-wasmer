#! /usr/bin/env bash
cargo test
cargo test --manifest-path test/Cargo.toml

cargo test-fuzz