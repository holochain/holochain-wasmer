#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer-wasmi

# it's possible to flamegraph the benchmarks like this:
#
# cd test
# flamegraph cargo bench --bench bench -- --profile-time 10
