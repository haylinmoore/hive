{
  config,
  lib,
  pkgs,
  modulesPath,
  hive,
  ...
}:
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
  boot.kernelPackages = pkgs.linuxPackages_6_18;

  boot.initrd.luks.devices = {
    "luks-febbe92f-bb86-4f38-a981-bf5d86e5c9c0".device =
      "/dev/disk/by-uuid/febbe92f-bb86-4f38-a981-bf5d86e5c9c0";
    "luks-f7188184-8815-456c-9242-a3e7b976d987".device =
      "/dev/disk/by-uuid/f7188184-8815-456c-9242-a3e7b976d987";
  };

  fileSystems."/" = {
    device = "/dev/mapper/luks-febbe92f-bb86-4f38-a981-bf5d86e5c9c0";
    fsType = "btrfs";
  };

  fileSystems."/home" = {
    device = "/dev/mapper/luks-febbe92f-bb86-4f38-a981-bf5d86e5c9c0";
    fsType = "btrfs";
    options = [ "subvol=home" ];
  };

  fileSystems."/nix" = {
    device = "/dev/mapper/luks-febbe92f-bb86-4f38-a981-bf5d86e5c9c0";
    fsType = "btrfs";
    options = [ "subvol=nix" ];
  };

  fileSystems."/boot" = {
    device = "/dev/disk/by-uuid/A512-4D6C";
    fsType = "vfat";
    options = [
      "fmask=0077"
      "dmask=0077"
    ];
  };

  swapDevices = [ { device = "/dev/mapper/luks-f7188184-8815-456c-9242-a3e7b976d987"; } ];
  boot.resumeDevice = "/dev/mapper/luks-f7188184-8815-456c-9242-a3e7b976d987";

  networking.useDHCP = lib.mkDefault true;

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  hardware.cpu.intel.npu.enable = true;
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
