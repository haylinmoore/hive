{
  config,
  lib,
  pkgs,
  ...
}:

let
  mkProxySite = name: cfg: {
    inherit name;
    value = {
      forceSSL = cfg.ssl;
      enableACME = cfg.ssl;
      locations = {
        "/" = {
          proxyPass = "${cfg.proxyUri}";
          extraConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto https;
          '';
        };
      };
    };
  };
in

{
  options.proxySites = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          domain = lib.mkOption {
            type = lib.types.str;
            description = "The domain for this proxy site.";
          };

          proxyUri = lib.mkOption {
            type = lib.types.str;
            description = "The connection Uri for this proxy site.";
          };

          ssl = lib.mkOption {
            type = lib.types.bool;
            default = true;
            description = "Whether to enable SSL for this proxy site.";
          };
        };
      }
    );
    default = { };
    description = "Proxy sites to serve with nginx.";
  };

  config = lib.mkIf (config.proxySites != { }) {
    services.nginx.enable = true;
    services.nginx.virtualHosts = lib.listToAttrs (
      lib.mapAttrsToList (name: cfg: mkProxySite cfg.domain cfg) config.proxySites
    );
  };
}
