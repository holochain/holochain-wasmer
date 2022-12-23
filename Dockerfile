FROM nixos/nix
ADD shell.nix ./shell.nix
ADD nix ./nix
RUN nix-shell --run "cargo install cargo-test-fuzz afl"
ADD crates ./crates
ADD Cargo.toml Cargo.toml
RUN nix-shell --run "cargo test"
RUN nix-shell --run "cargo test --manifest-path crates/guest/Cargo.toml"
ADD test ./test
RUN nix-shell --run "cargo test --manifest-path test/Cargo.toml"
ADD fuzz.sh ./fuzz.sh
CMD nix-shell --run ./fuzz.sh