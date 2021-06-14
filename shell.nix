let
  holonixPath = builtins.fetchTarball {
    url = "https://github.com/holochain/holonix/archive/2b7797d9ec3dc49f7e4b1e89d165f0d714d837e2.tar.gz";
    sha256 = "1q1z3mrdr0cf5p4nayc1lwq56n1080mkjzny44p2yzjpp8mydj66";
  };

  holonix = import (holonixPath) {
    includeHolochainBinaries = false;
  };

in holonix.main
