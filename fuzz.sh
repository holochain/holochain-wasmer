#! /usr/bin/env bash

cargo install cargo-test-fuzz afl

cd test
cargo test
cargo test-fuzz process_string_test