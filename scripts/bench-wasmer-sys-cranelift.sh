#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer-sys-cranelift

# it's possible to flamegraph the benchmarks like this:
#
# flamegraph cargo bench --bench bench -- --profile-time 10
