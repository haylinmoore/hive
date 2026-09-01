{
  pkgs,
  lib,
  ...
}:

pkgs.buildGoModule {
  pname = "tunnl";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  vendorHash = "sha256-ZHomKlznHPsnfZXRy3jY6oqSgjkcVykiB3FgAsO0IZc=";

  subPackages = [ "cmd/tunnl" ];

  meta.mainProgram = "tunnl";
}
