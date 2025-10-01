{
  config,
  lib,
  pkgs,
  modulesPath,
  hive,
  ...
}:
let
  pinnedNixpkgs = import (builtins.fetchTarball {
    url = "https://github.com/nixos/nixpkgs/archive/94def634a20494ee057c76998843c015909d6311.tar.gz";
    sha256 = "1cnb74cr9zz88430xy1m9f7dxyx6237qwpljnqy62m6xjx264r9b";
  }) { inherit (pkgs) system config; };
in
{
  imports = [ (modulesPath + "/installer/scan/not-detected.nix") ];

  boot.initrd.availableKernelModules = [
    "xhci_pci"
    "thunderbolt"
    "nvme"
    "usb_storage"
    "sd_mod"
  ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [
    "kvm-intel"
    "xe"
  ];
  boot.extraModulePackages = [ ];
  boot.kernelPackages = pinnedNixpkgs.linuxPackages_6_16;

  fileSystems."/" = {
    device = "/dev/disk/by-uuid/a896eea0-94aa-4a66-8ce7-2a2ba81b5f64";
    fsType = "ext4";
  };

  fileSystems."/boot" = {
    device = "/dev/disk/by-uuid/DF83-D575";
    fsType = "vfat";
    options = [
      "fmask=0077"
      "dmask=0077"
    ];
  };

  swapDevices = [ { device = "/dev/disk/by-uuid/d586ca21-878d-4310-9338-efad26883a3c"; } ];

  # Enables DHCP on each ethernet and wireless interface. In case of scripted networking
  # (the default) this is the recommended approach. When using systemd-networkd it's
  # still possible to use this option, but it's recommended to use it in conjunction
  # with explicit per-interface declarations with `networking.interfaces.<interface>.useDHCP`.
  networking.useDHCP = lib.mkDefault true;
  # networking.interfaces.wlo1.useDHCP = lib.mkDefault true;

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
