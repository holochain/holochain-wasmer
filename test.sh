export RUST_BACKTRACE=full
cargo fmt
cargo test
cargo build --release --manifest-path test/wasm/Cargo.toml --target wasm32-unknown-unknown -Z unstable-options \
&& cargo test --manifest-path test/Cargo.toml -- --nocapture
