#! /usr/bin/env bash
(
 cd test && \
 cargo bench --no-default-features --features wasmer_wamr
)

# it's possible to flamegraph the benchmarks like this:
#
# cd test
# flamegraph cargo bench --bench bench -- --profile-time 10
