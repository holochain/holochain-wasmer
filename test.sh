export RUST_BACKTRACE=full
export WASMER_BACKTRACE=1
cargo fmt
( cd test && cargo fmt )
cargo test
cargo build --release --manifest-path test/test_wasm/Cargo.toml --target wasm32-unknown-unknown -Z unstable-options \
&& cargo test --manifest-path test/Cargo.toml ${1} -- --nocapture
