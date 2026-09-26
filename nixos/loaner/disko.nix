{ config, hive, ... }:
let
  btrfs = mountpoint: {
    inherit mountpoint;
    mountOptions = [
      "compress=zstd"
      "noatime"
    ];
  };
in
{
  imports = [ "${hive.sources.disko}/module.nix" ];

  disko.devices.disk.main = {
    type = "disk";
    device = config.loaner.disk;
    content = {
      type = "gpt";
      partitions = {
        ESP = {
          size = "1G";
          type = "EF00";
          content = {
            type = "filesystem";
            format = "vfat";
            mountpoint = "/boot";
            mountOptions = [ "umask=0077" ];
          };
        };
        luks = {
          size = "100%";
          content = {
            type = "luks";
            name = "crypted";
            passwordFile = "/tmp/disk.key";
            settings = {
              allowDiscards = true;
              crypttabExtraOpts = [ "tpm2-device=auto" ];
            };
            content = {
              type = "btrfs";
              extraArgs = [ "-f" ];
              subvolumes = {
                "/root" = btrfs "/";
                "/nix" = btrfs "/nix";
                "/persistent" = btrfs "/persistent";
                "/guest" = btrfs "/home/${config.loaner.guest.name}";
              };
            };
          };
        };
      };
    };
  };

  fileSystems."/persistent".neededForBoot = true;
}
