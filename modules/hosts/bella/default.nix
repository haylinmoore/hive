{
  config,
  lib,
  pkgs,
  ...
}:

let
  sources = import ../../../npins;
in

{
  deployment.targetHost = "bella.infra.hayl.in";

  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [
    80
    443
    2222
  ];

  networking.firewall.allowedUDPPorts = [
    80
    443
  ];

  imports = [
    ../../shared/pve.nix
    ./hardware-configuration.nix
    ./networking.nix
    ../../services/yggdrasil.nix
    ../../certs/hayl-in.nix
    ../../certs/estrogen-coffee.nix
  ];

  virtualisation.podman.enable = true;

  sops.secrets."dns" = {
    sopsFile = ../../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [
      "acme-hayl.in.service"
      "acme-estrogen.coffee.service"
    ];
  };
}
