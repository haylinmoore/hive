{
  config,
  lib,
  pkgs,
  ...
}:

let
  mkStaticSite = name: cfg: {
    inherit name;
    value = {
      forceSSL = cfg.ssl;
      enableACME = cfg.ssl && cfg.useACMEHost == null;
      useACMEHost = cfg.useACMEHost;
      root = import cfg.source { inherit pkgs; };
    };
  };
in

{
  options.staticSites = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          domain = lib.mkOption {
            type = lib.types.str;
            description = "The domain for this static site.";
          };

          source = lib.mkOption {
            type = lib.types.path;
            description = "The source path/derivation for this static site.";
          };

          ssl = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Whether to enable SSL for this static site.";
          };

          useACMEHost = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Use an existing ACME certificate from the specified host instead of generating a new one.";
          };
        };
      }
    );
    default = { };
    description = "Static sites to serve with nginx.";
  };

  config = lib.mkIf (config.staticSites != { }) {
    services.nginx.enable = true;
    services.nginx.virtualHosts = lib.listToAttrs (
      lib.mapAttrsToList (name: cfg: mkStaticSite cfg.domain cfg) config.staticSites
    );
  };
}
