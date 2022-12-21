#! /usr/bin/env bash

cargo install cargo-test-fuzz afl
cargo test
cargo test-fuzz round_trip