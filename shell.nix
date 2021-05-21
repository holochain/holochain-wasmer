let
  holonixPath = builtins.fetchTarball {
    url = "https://github.com/holochain/holonix/archive/f8ee43113ababeedad3e1203a8e8309f5d75e3f8.tar.gz";
    sha256 = "1765x81nkm79fn98kv782wr6amj2vaxccp9l4ppv8h1lhfvapbgp";
  };

  holonix = import (holonixPath) {
    includeHolochainBinaries = false;
  };

in holonix.main