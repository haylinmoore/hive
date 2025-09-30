{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

let
  impermanence = hive.sources.impermanence.outPath;
in
{
  imports = [
    "${impermanence}/nixos.nix"
  ];

  environment.persistence."/persistent" = {
    hideMounts = true;
    directories = [
      "/var/log"
      "/var/lib/nixos"
      "/var/lib/systemd/coredump"
      "/etc/NetworkManager/system-connections"
      "/var/lib/acme"
    ];
    files = [
      "/etc/machine-id"
      "/etc/ssh/ssh_host_ed25519_key"
      "/etc/ssh/ssh_host_ed25519_key.pub"
      "/etc/ssh/ssh_host_rsa_key"
      "/etc/ssh/ssh_host_rsa_key.pub"
    ];
    users.root = {
      home = "/root";
    };
  };
}
