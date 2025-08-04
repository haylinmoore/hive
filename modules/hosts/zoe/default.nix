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
    ../../nixos/dli-ddns.nix
  ];

  networking.firewall.enable = false;

  # Add meshcore-cli for system-wide use
  environment.systemPackages = with pkgs; [
    (callPackage ../../packages/meshcore-cli.nix { })
  ];

  sops.secrets."dns" = {
    sopsFile = ../../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [ "acme-uwu.estate.service" ];
  };

  services.dli-ddns.zoe-infra-hayl-in = {
    provider = "bunny";
    zone = "hayl.in";
    record = "zoe.infra";
    credentialsFile = "/run/secrets/dns";
    interval = "*:0/5";
    records = {
      A = {
        mode = "external";
      };
      AAAA = {
        mode = "interface";
        interface = "enp4s0";
        pattern = "^2.*243$";
      };
    };
  };
}
