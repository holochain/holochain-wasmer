{
  description = "Flake for Holochain app development";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, rust-overlay, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
    perSystem = { system, ... }: let
      pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    in {
      formatter = pkgs.nixpkgs-fmt;

      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain
          bzip2
          # These packages and env vars are required to build Wasmer with the 'wamr' feature
          cmake
          clang
          llvmPackages.libclang.lib
          ninja
          # These packages are required to build Wasmer with the production config
          llvm_18
          llvmPackages_18.libunwind
          libffi
          libxml2
          zlib
          ncurses
        ];
        # Used by `wamr`
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        # Used by wasmer production config
        shellHook = ''
          # This binary lives in a different derivation to `llvm_18` and isn't re-exported through that derivation
          export LLVM_SYS_180_PREFIX=$(which llvm-config | xargs dirname | xargs dirname)
          export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib:${pkgs.zlib}/lib:${pkgs.ncurses}/lib"
        '';
      };
    };
  };
}
