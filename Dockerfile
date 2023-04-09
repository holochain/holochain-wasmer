FROM holochain/fuzzbox:base

ADD . .

RUN nix-shell --run "cargo test"
RUN nix-shell --run "cargo test --manifest-path crates/guest/Cargo.toml"
RUN nix-shell --run "cargo test --manifest-path test/Cargo.toml"

ENTRYPOINT ["nix-shell", "--run", "./fuzz.sh"]