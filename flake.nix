{
  description = "Flake to provide a development shell with helpful libraries and tools";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
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
          # `wasmi` feature is enabled — it runs bindgen against the wasmi C API
          # headers to generate Rust bindings.
          clang
          llvmPackages.libclang.lib
          # These packages are required to build Wasmer with the production config.
          # Wasmer 7.x links against LLVM 21 via llvm-sys 211.
          llvm_21
          llvmPackages_21.libunwind
          libffi
          libxml2
          zlib
          ncurses
        ];
        # Used by bindgen when wasmer is built with the `wasmi` feature.
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        # Used by wasmer production config. Point llvm-sys directly at the
        # LLVM 21 dev output so we are not at the mercy of PATH ordering with
        # `clang` (which may bring its own llvm-config).
        LLVM_SYS_211_PREFIX = "${pkgs.llvmPackages_21.llvm.dev}";
        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib:${pkgs.zlib}/lib:${pkgs.ncurses}/lib"
        '';
      };
    };
  };
}
