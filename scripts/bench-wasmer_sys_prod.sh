#! /usr/bin/env bash
(
 cd test-crates/test && \
 cargo bench --no-default-features --features wasmer_sys_prod
)
