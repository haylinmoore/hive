{ lib, ... }:

{
  options.persist = {
    directories = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        Directories a service needs kept across boots. Ignored on hosts
        without impermanence, so services can declare this unconditionally.
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
