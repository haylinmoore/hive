{ lib, ... }:
{
  deployment.targetHost = "192.168.42.171";
  deployment.buildOnTarget = lib.mkForce false;

  imports = [
    ./hardware-configuration.nix
    ./platform.nix
    ../../nixos/loaner
  ];

  networking.hostName = "sasha";
  loaner.disk = "/dev/nvme0n1";

  system.stateVersion = "26.05";
}
