{
  config,
  lib,
  pkgs,
  ...
}:

let
  haytop = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop";
  qumulo = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILXL2etr9int91HoWiyh8P4+UJH/nb2i5KSJ+tBb+BDi hmoore@qumulo.com";
in
{
  options.sshKeys = {
    admin = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      description = "Keys trusted with a root shell.";
    };

    tunnl = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      description = ''
        Keys allowed to open a tunnel. Opening a tunnel only exposes a port
        the client already chose to forward, so this can be looser than
        admin, and a key here gets no shell anywhere.
      '';
    };
  };

  config = {
    sshKeys.admin = [ haytop ];

    # The work laptop should be able to expose a local port without being
    # trusted with a root shell on the infrastructure.
    sshKeys.tunnl = config.sshKeys.admin ++ [ qumulo ];

    users.users = {
      root = {
        openssh.authorizedKeys.keys = config.sshKeys.admin;
      };
      haylin = {
        isNormalUser = true;
        extraGroups = [ "wheel" ];
        packages = with pkgs; [
          tree
        ];
        openssh.authorizedKeys.keys = [ haytop ];
      };
    };

    security.sudo.wheelNeedsPassword = false;
  };
}
