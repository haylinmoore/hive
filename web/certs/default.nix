{
  pkgs,
  hive,
  lib,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };

  service =
    package:
    inputs@{
      bind,
      dir,
      zones,
    }:
    hive.service.lib.mkServiceDerivation {
      name = "certs";

      inherit package;
      command = "${package}/bin/certs serve";

      user = "certs";
      group = "certs";

      environment.CERTS_BIND = bind;
      environment.CERTS_DIR = dir;
      environment.CERTS_ZONES = lib.concatStringsSep "," zones;

      meta = {
        inherit bind dir zones;
        inherit inputs;
      };
    };

  package = naersk.buildPackage {
    src = lib.cleanSourceWith {
      src = ./.;
      filter = path: type: !(type == "directory" && baseNameOf (toString path) == "target");
    };

    passthru = {
      service = service package;
    };
  };
in

package
