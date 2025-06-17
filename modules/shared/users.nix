{
  config,
  lib,
  pkgs,
  ...
}:

{
  users.users = {
    root = {
      openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICKsUHsNfWi9qEivDXP146uGBnW2H1m4tOW+An0b3MkZ infra@hayl.in"
      ];
    };
    haylin = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      packages = with pkgs; [
        tree
      ];
      openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"
      ];
    };
  };

  security.sudo.wheelNeedsPassword = false;
}
