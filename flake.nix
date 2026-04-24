{
  description = "Flake to provide a development shell with helpful libraries and tools";

  inputs = {
    # nixos-unstable rather than nixos-25.11 because LLVM 22 shipped
    # after the 25.11 freeze. wasmer 7.2.x links against llvm-sys 221
    # (LLVM 22) via `wasmer-sys-llvm`, so llvmPackages_22 must be
    # reachable from the flake.
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
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
          # clang + libclang are required by wasmer's build script when the
          # `v8` feature is enabled — it runs bindgen against the wasm-c-api
          # header and compiles a small C++ shim with the `cc` crate.
          clang
          llvmPackages.libclang.lib
          # These packages are required to build Wasmer with the production config.
          # Wasmer 7.2.x links against LLVM 22 via llvm-sys 221.
          llvm_22
          llvmPackages_22.libunwind
          libffi
          libxml2
          zlib
          ncurses
        ];
        # Used by bindgen when wasmer is built with the `v8` feature.
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        # Used by wasmer production config. Point llvm-sys directly at the
        # LLVM 22 dev output so we are not at the mercy of PATH ordering with
        # `clang` (which may bring its own llvm-config).
        LLVM_SYS_221_PREFIX = "${pkgs.llvmPackages_22.llvm.dev}";
        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib:${pkgs.zlib}/lib:${pkgs.ncurses}/lib"
        '';
      };
    };
  };
}
