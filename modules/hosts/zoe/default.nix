{
  config,
  lib,
  pkgs,
  ...
}:

{
  deployment.targetHost = "zoe.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ../../services/home-assistant.nix
  ];

  networking.firewall.enable = false;

  services.duckdns-ds = {
    enable = true;
    tokenFile = "/run/secrets/duckdns";
    domains = [
      "uwu-estate"
    ];
    ipv6Suffix = "243";
  };

  sops.secrets."duckdns" = {
    sopsFile = ../../../secrets/zoe/tokens.yaml;
    key = "duckdns";
    owner = config.systemd.services.duckdns-ds.serviceConfig.User;
    restartUnits = [ "duckdns-ds.service" ];
  };
}
