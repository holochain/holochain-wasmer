#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer_sys_prod
