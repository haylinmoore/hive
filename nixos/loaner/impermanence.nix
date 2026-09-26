{ config, hive, ... }:
{
  imports = [ "${hive.sources.impermanence}/nixos.nix" ];

  boot.initrd.systemd.services.rollback = {
    description = "Start from an empty root subvolume";
    wantedBy = [ "initrd.target" ];
    requires = [ "systemd-cryptsetup@crypted.service" ];
    after = [ "systemd-cryptsetup@crypted.service" ];
    before = [ "sysroot.mount" ];
    unitConfig.DefaultDependencies = "no";
    serviceConfig.Type = "oneshot";
    script = ''
      mkdir /btrfs_tmp
      mount -o subvol=/ /dev/mapper/crypted /btrfs_tmp
      if [[ -e /btrfs_tmp/root ]]; then
        btrfs subvolume list -o /btrfs_tmp/root | cut -f 9- -d ' ' | sort -r | while read -r sub; do
          btrfs subvolume delete "/btrfs_tmp/$sub"
        done
        btrfs subvolume delete /btrfs_tmp/root
      fi
      btrfs subvolume create /btrfs_tmp/root
      umount /btrfs_tmp
    '';
  };

  environment.persistence."/persistent" = {
    hideMounts = true;
    directories = [
      "/var/log"
      "/var/lib/nixos"
      "/var/lib/systemd/coredump"
      "/var/lib/bluetooth"
      "/etc/NetworkManager/system-connections"
    ]
    ++ config.persist.directories;
    files = [
      "/etc/machine-id"
      "/etc/ssh/ssh_host_ed25519_key"
      "/etc/ssh/ssh_host_ed25519_key.pub"
      "/etc/ssh/ssh_host_rsa_key"
      "/etc/ssh/ssh_host_rsa_key.pub"
    ]
    ++ config.persist.files;
  };
}
