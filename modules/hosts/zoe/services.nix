{
  config,
  lib,
  pkgs,
  ...
}:

{

  services.duckdns-ds = {
    enable = true;
    tokenFile = "/run/secrets/duckdns";
    domains = [
      "uwu-estate"
    ];
  };
}
