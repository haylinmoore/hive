{ config, lib, pkgs, ... }:

let
  sources = import ../../npins;
  dollpublish_pkg = import sources.dollpublish { inherit pkgs; };
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
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "Mapping of domain aliases to usernames.";
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
        enableACME = true;
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
    } // (lib.mapAttrs (domain: username: {
      forceSSL = true;
      enableACME = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:${toString config.dollpublish.port}/${username}/";
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
