{ config, lib, pkgs, ... }:

let
  mkStaticSite = name: cfg: {
    inherit name;
    value = {
      forceSSL = true;
      enableACME = true;
      root = import cfg.source { inherit pkgs; };
    };
  };
in

{
  options.staticSites = lib.mkOption {
    type = lib.types.attrsOf (lib.types.submodule {
      options = {
        domain = lib.mkOption {
          type = lib.types.str;
          description = "The domain for this static site.";
        };
        
        source = lib.mkOption {
          type = lib.types.path;
          description = "The source path/derivation for this static site.";
        };
      };
    });
    default = {};
    description = "Static sites to serve with nginx.";
  };

  config = lib.mkIf (config.staticSites != {}) {
    services.nginx.enable = true;
    services.nginx.virtualHosts = lib.listToAttrs (
      lib.mapAttrsToList (name: cfg: mkStaticSite cfg.domain cfg) config.staticSites
    );
  };
}
