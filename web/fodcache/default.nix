{
  pkgs,
  hive,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };

  service =
    package:
    inputs@{
      bind,
      stateDir,
      maxSize,
    }:
    hive.service.lib.mkServiceDerivation {
      name = "fodcache";

      inherit package;
      command = "${package}/bin/fodcache";

      user = "fodcache";
      group = "fodcache";

      environment = {
        FODCACHE_BIND = bind;
        FODCACHE_STATE = stateDir;
        FODCACHE_MAX_SIZE = toString maxSize;
      };

      meta = {
        inherit inputs;
      };
    };

  package = naersk.buildPackage {
    src = ./.;

    passthru = {
      service = service package;
    };
  };
in

package
