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

                # These packages and env vars are required to build wasmer on the 'wamr' branch (i.e. the hc-wasmer dependency)
                packages = [
                  pkgs.cmake
                  pkgs.clang
                  pkgs.llvmPackages.libclang.lib
                ];
                LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
            };
        };
    };
}
