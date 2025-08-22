{
  config,
  lib,
  pkgs,
  ...
}:

{
  deployment.targetHost = "athena.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./impermanence.nix
    ./bgp.nix
    ../../certs/hayl-in.nix
    ../../services/www.nix
  ];

  security.tpm2.enable = false;

  networking.firewall.enable = false;

  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = false;
    };
  };

  environment.systemPackages = with pkgs; [
    ethtool
    ipmitool
    mtr
    bird3
  ];

  sops.secrets."dns" = {
    sopsFile = ../../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [
      "acme-hayl.in.service"
    ];
  };

  system.stateVersion = "25.11";
}
