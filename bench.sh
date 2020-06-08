#! /usr/bin/env bash
(
 cd test && \
 cargo bench
)
# flamegraph cargo bench --bench allocation -- --profile-time 10
