export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1
cargo fmt
( cd test && cargo fmt )
( cd crates/guest && cargo fmt )

# tests the root workspace that doesn't include any wasm code
cargo test -- --nocapture

# cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown -Z unstable-options

# build wasm and run the "full" tests
cargo test --manifest-path test/Cargo.toml ${1} -- --nocapture
