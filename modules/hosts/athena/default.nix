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

  system.stateVersion = "25.11";
}
