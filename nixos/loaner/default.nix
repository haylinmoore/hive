{ lib, ... }:
{
  imports = [
    ./disko.nix
    ./impermanence.nix
    ./guest.nix
    ./desktop.nix
    ./maintenance.nix
  ];

  options.loaner = {
    disk = lib.mkOption {
      type = lib.types.str;
      example = "/dev/nvme0n1";
      description = "Disk the loaner is installed to. Everything on it is erased by the install.";
    };

    guest.name = lib.mkOption {
      type = lib.types.str;
      default = "me";
      description = "Username of the account handed to whoever has the laptop.";
    };

    guest.defaultHashedPassword = lib.mkOption {
      type = lib.types.str;
      default = "$y$j9T$6SohYO6eAokI23YMMVfVD0$H26qsv/J4.jkrWZiublHTgy4t1Jv7PhMbZ8oXaSQJi5";
      description = ''
        Hash the password is reset to, "password" by default. A fresh account is
        made to pick its own password on first login, and SSH only runs until then.
      '';
    };
  };
}
