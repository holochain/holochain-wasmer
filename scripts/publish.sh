#!/usr/bin/env nix-shell
#! nix-shell ../shell.nix
#! nix-shell -i bash

set -xe

cargo publish --manifest-path=crates/common/Cargo.toml && \
  cargo publish --manifest-path=crates/host/Cargo.toml && \
  cargo publish --manifest-path=crates/guest/Cargo.toml
