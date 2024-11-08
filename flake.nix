{
  description = "Flake for Holochain app development";

  inputs = {
    versions.url = "github:holochain/holochain?dir=versions/weekly";
    holochain-flake = {
      url = "github:holochain/holochain";
      inputs.versions.follows = "versions";
    };

    nixpkgs.follows = "holochain-flake/nixpkgs";
  };

  outputs = inputs @ { ... }:
    inputs.holochain-flake.inputs.flake-parts.lib.mkFlake { inherit inputs; }
    {
        systems = builtins.attrNames inputs.holochain-flake.devShells;
        perSystem = { config, pkgs, system, ... }: {
            devShells.default = pkgs.mkShell {
                inputsFrom = [
                    inputs.holochain-flake.devShells.${system}.rustDev
                ];

                packages = with pkgs; [
                  # These packages and env vars are required to build Wasmer with the 'wamr' feature
                  cmake
                  clang
                  llvmPackages.libclang.lib
                  # These packages are required to build Wasmer with the production config
                  llvm_15
                  libffi
                  libxml2
                  zlib
                  ncurses
                ];
                # Used by `wamr`
                LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
                # Used by wasmer production config
                shellHook = ''
                    # This binary lives in a different derivation to `llvm_15` and isn't re-exported through that derivation
                    export LLVM_SYS_150_PREFIX=$(which llvm-config | xargs dirname | xargs dirname)
                    export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libffi}/lib:${pkgs.zlib}/lib:${pkgs.ncurses}/lib"
                '';
            };
        };
    };
}
