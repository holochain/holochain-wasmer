cargo build --release --manifest-path guest/Cargo.toml --target wasm32-unknown-unknown -Z unstable-options \
&& cargo test --manifest-path host/Cargo.toml -- --nocapture
