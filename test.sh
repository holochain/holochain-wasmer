set -euxo pipefail

export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1
cargo fmt
( cd test && cargo fmt )
( cd crates/guest && cargo fmt )

cargo clippy
( cd test && cargo clippy )
( cd crates/guest && cargo clippy )

# tests the root workspace that doesn't include any wasm code
cargo test ${1-} -- --nocapture

# test that everything builds
cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown

# build wasm and run the "full" tests
cargo test --release --manifest-path test/Cargo.toml ${1-} -- --nocapture
