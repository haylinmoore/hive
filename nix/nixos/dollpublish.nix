{
  config,
  lib,
  pkgs,
  mono,
  ...
}:

let
  dollpublish_pkg = import mono.sources.dollpublish { inherit pkgs; };
in

{
  options.dollpublish = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable the dollpublish service.";
    };

    dataDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/dollpublish";
      description = "Directory where dollpublish will write data.";
    };

    bindAddr = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "The address to bind the server to.";
    };

    port = lib.mkOption {
      type = lib.types.int;
      default = 3000;
      description = "The port on which the server will listen.";
    };

    domain = lib.mkOption {
      type = lib.types.str;
      default = "dollpublish.tld";
      description = "The domain for the dollpublish service.";
    };

    domainAliases = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            username = lib.mkOption {
              type = lib.types.str;
              description = "The username for this domain alias.";
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
      description = "Mapping of domain aliases to username and ACME configuration.";
    };

    useACMEHost = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Use an existing ACME certificate from the specified host instead of generating a new one.";
    };
  };

  config = lib.mkIf config.dollpublish.enable {
    users.users.dollpublish = {
      isNormalUser = true;
      description = "User for running dollpublish service";
      home = config.dollpublish.dataDir;
      createHome = true;
      extraGroups = [ "nobody" ];
    };

    systemd.services.dollpublish = {
      description = "dollpublish service";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        User = "dollpublish";
        Group = "nobody";
        Environment = [
          "MOON_DATA_DIR=${config.dollpublish.dataDir}"
          "MOON_BIND_ADDR=${config.dollpublish.bindAddr}"
          "MOON_PORT=${toString config.dollpublish.port}"
        ];
        ExecStart = "${dollpublish_pkg}/bin/dollpublish";
        Restart = "always";
      };
    };

    environment.etc."dollpublish".source = config.dollpublish.dataDir;

    services.nginx.enable = true;
    services.nginx.virtualHosts = {
      "${config.dollpublish.domain}" = {
        forceSSL = true;
        enableACME = config.dollpublish.useACMEHost == null;
        useACMEHost = config.dollpublish.useACMEHost;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString config.dollpublish.port}/";
          extraConfig = ''
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto https;
          '';
        };
      };
    }
    // (lib.mapAttrs (domain: cfg: {
      forceSSL = true;
      enableACME = cfg.useACMEHost == null;
      useACMEHost = cfg.useACMEHost;
      locations."/" = {
        proxyPass = "http://127.0.0.1:${toString config.dollpublish.port}/${cfg.username}/";
        extraConfig = ''
          proxy_set_header Host $host;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
          proxy_set_header X-Forwarded-Proto https;
        '';
      };
    }) config.dollpublish.domainAliases);
  };
}
