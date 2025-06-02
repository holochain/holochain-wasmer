#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer_v8

# it's possible to flamegraph the benchmarks like this:
#
# cd test
# flamegraph cargo bench --bench bench -- --profile-time 10
