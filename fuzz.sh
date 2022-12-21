#! /usr/bin/env bash

cd crates/common
cargo install cargo-test-fuzz afl
cargo test
cargo test-fuzz round_trip