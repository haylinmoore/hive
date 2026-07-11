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
      domain,
      dialUri,
      panelUri,
      dbPath,
    }:
    hive.service.lib.mkServiceDerivation {
      name = "dial-${domain}";

      inherit package;
      command = "${package}/bin/dial";

      user = "dial-service";
      group = "dial-service";

      environment.DIAL_BIND = dialUri;
      environment.DB_PATH = dbPath;
      environment.PANEL_BIND = panelUri;

      # Metadata with both convenience fields and full inputs
      meta = {
        inherit
          domain
          dialUri
          panelUri
          dbPath
          ;
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
