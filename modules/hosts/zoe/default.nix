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
    ../../certs/uwu-estate.nix
    ../../nixos/dynamic-dns.nix
  ];

  networking.firewall.enable = false;

  sops.secrets."dns" = {
    sopsFile = ../../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [ "acme-uwu.estate.service" ];
  };

  services.dynamic-dns.zoe-infra-hayl-in = {
    provider = "bunny";
    zone = "hayl.in";
    record = "zoe.infra.";
    credentialsFile = "/run/secrets/dns";
    interval = "*:0/5";
    ipv6Suffix = "243";
    interface = "enp4s0";
  };
}
