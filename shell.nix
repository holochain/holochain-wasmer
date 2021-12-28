let
  holonixPath = (import ./nix/sources.nix).holonix;

  holonix = import (holonixPath) {
    include = {
      holochainBinaries = false;
    };
  };

in holonix.main
