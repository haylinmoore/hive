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

  imports = [
    ../../shared/pve.nix
    ./hardware-configuration.nix
    ./networking.nix
    ./music
    ../../services/soft-serve.nix
    ../../services/dollpublish.nix
    ../../services/www.nix
    ../../services/yggdrasil.nix
    ../../services/mysql.nix
    ../../services/lambda.nix
    ../../services/88x31.nix
    ../../services/256.nix
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
