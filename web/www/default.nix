{
  pkgs,
  hive,
  lib,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk { };

  service =
    package:
    inputs@{
      domain ? "hayl.in",
      port ? 15641,
      bindAddr ? "127.0.0.1",
    }:
    hive.service.lib.mkServiceDerivation {
      name = "www-${domain}";

      inherit package;
      command = "${package}/bin/www";

      user = "nobody";
      group = "nobody";
      workingDirectory = "${package}/";

      environment.BIND = "${bindAddr}:${toString port}";

      # Metadata with both convenience fields and full inputs
      meta = {
        inherit domain port bindAddr;
        inherit inputs;
      };
    };

  package = naersk.buildPackage {
    src = ./.;

    nativeBuildInputs = with pkgs; [
      image_optim
    ];

    postInstall = ''
      mkdir -p $out/assets
      mkdir -p $out/content
      cp -r ./assets/* $out/assets/

      image_optim -r $out/assets/

      cp -r ./content/* $out/content/
    '';

    passthru = {
      service = service package;
    };
  };
in

package
