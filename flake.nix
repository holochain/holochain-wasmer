{
  description = "Flake for Holochain app development";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";
  };

  outputs = inputs@{ flake-parts, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, ... }: {
      formatter = pkgs.nixpkgs-fmt;

      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # These packages and env vars are required to build Wasmer with the 'wamr' feature
          cmake
          clang
          mold
          llvmPackages.libclang.lib
          ninja
          # These packages are required to build Wasmer with the production config
          llvm_18
          llvmPackages_18.libunwind
          libffi
          libxml2
          zlib
          ncurses
        ] ++ [ inputs'.holonix.packages.rust ];
        # Used by `wamr`
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        # Used by wasmer production config
        shellHook = ''
          # This binary lives in a different derivation to `llvm_15` and isn't re-exported through that derivation
          export LLVM_SYS_180_PREFIX=$(which llvm-config | xargs dirname | xargs dirname)
          export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib:${pkgs.zlib}/lib:${pkgs.ncurses}/lib"
        '';
      };
    };
  };
}
