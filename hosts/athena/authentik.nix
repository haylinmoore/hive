{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
let
  authentik-nix = import hive.sources.authentik-nix;
in
{
  imports = [
    authentik-nix.nixosModules.default
  ];

  sops.secrets."authentik" = {
    sopsFile = ../../secrets/authentik.env;
    key = "";
    format = "dotenv";
    restartUnits = [ "authentik.service" ];
  };

  environment.persistence."/persistent".directories = [
    "/var/lib/private/authentik"
    "/var/lib/postgresql"
  ];

  services.authentik = {
    enable = true;
    environmentFile = "/run/secrets/authentik";
    settings = {
      disable_startup_analytics = true;
      avatars = "initials";
    };
    nginx = {
      enable = true;
      host = "auth.hayl.in";
    };
  };

  services.nginx.virtualHosts."auth.hayl.in" = {
    useACMEHost = "hayl.in";
    forceSSL = lib.mkForce true;
  };
}
