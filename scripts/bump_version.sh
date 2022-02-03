#!/usr/bin/env nix-shell
#! nix-shell ../shell.nix
#! nix-shell -i bash

set -xe

FROM_VERSION="${1:?}"
TO_VERSION="${2:?}"

find crates -name Cargo.toml -exec sed -i "s,${FROM_VERSION},${TO_VERSION},g" {} \;
find crates -name Cargo.toml -exec cargo check --manifest-path={} \;
find test -name Cargo.toml -exec sed -i "s,${FROM_VERSION},${TO_VERSION},g" {} \;
find test -name Cargo.toml -exec cargo check --manifest-path={} \;
./test.sh
git commit crates test -m "bumping versions from ${FROM_VERSION} to ${TO_VERSION}"
