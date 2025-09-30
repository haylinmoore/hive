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
      enableACME = cfg.ssl && cfg.useACMEHost == null;
      useACMEHost = cfg.useACMEHost;
      http2 = true;
      http3 = true;
      quic = true;
      extraConfig = ''
        add_header Alt-Svc 'h3=":443"; ma=86400';
      '';
      locations = {
        "/" = {
          proxyPass = "${cfg.proxyUri}";
          proxyWebsockets = true;
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

          useACMEHost = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Use an existing ACME certificate from the specified host instead of generating a new one.";
          };
        };
      }
    );
    default = { };
    description = "Proxy sites to serve with nginx.";
  };

  config = lib.mkIf (config.proxySites != { }) {
    services.nginx.enable = true;
    #services.nginx.recommendedProxySettings = true;
    services.nginx.virtualHosts = lib.listToAttrs (
      lib.mapAttrsToList (name: cfg: mkProxySite cfg.domain cfg) config.proxySites
    );
  };
}
