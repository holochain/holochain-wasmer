#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer_sys_cranelift

# it's possible to flamegraph the benchmarks like this:
#
# flamegraph cargo bench --bench bench -- --profile-time 10
