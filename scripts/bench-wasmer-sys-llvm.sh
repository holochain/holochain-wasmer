#! /usr/bin/env bash

cargo bench -p tests --no-default-features --features wasmer-sys-llvm
