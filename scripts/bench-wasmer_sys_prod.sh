#! /usr/bin/env bash
(
 cd test && \
 cargo bench --no-default-features --features wasmer_sys_prod
)
