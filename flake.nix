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
            buildInputs = [
              pkgs.libffi
              pkgs.libxml2
              pkgs.llvmPackages_15.llvm
              pkgs.ncurses
            ];
            inputsFrom = [
              inputs.holochain-flake.devShells.${system}.rustDev
            ];
            packages = [ ];
            env = pkgs.lib.optionalAttrs pkgs.stdenv.cc.isClang {
              NIX_LDFLAGS = "-l${pkgs.stdenv.cc.libcxx.cxxabi.libName}";
            };
          };
        };
      };
}
