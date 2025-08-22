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

  system.stateVersion = "25.11";
}
