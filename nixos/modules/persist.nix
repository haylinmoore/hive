{ lib, ... }:

{
  options.persist = {
    directories = lib.mkOption {
      type = lib.types.listOf (lib.types.either lib.types.str lib.types.attrs);
      default = [ ];
      example = [
        "/var/lib/thing"
        {
          directory = "/var/lib/other";
          user = "other";
          group = "other";
          mode = "0755";
        }
      ];
      description = ''
        Directories a service needs kept across boots. Ignored on hosts
        without impermanence, so services can declare this unconditionally.

        A bare path is kept as root. A service running as its own user needs
        the attribute form, because the bind mount is created before tmpfiles
        would fix the ownership up.
      '';
    };

    files = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        Files a service needs kept across boots. Ignored on hosts without
        impermanence, so services can declare this unconditionally.
      '';
    };
  };
}
