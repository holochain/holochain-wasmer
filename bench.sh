#! /usr/bin/env bash
(
 cd test && \
 cargo bench
)

# it's possible to flamegraph the benchmarks like this:
#
# cd test
# flamegraph cargo bench --bench bench -- --profile-time 10
