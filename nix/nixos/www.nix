{
  config,
  lib,
  pkgs,
  mono,
  ...
}:

let
  www_pkg = import mono.sources.www { inherit pkgs; };
in

{
  options.www = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable the www service.";
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
      default = "hayl.in";
      description = "The domain for the www service.";
    };

    useACMEHost = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Use an existing ACME certificate from the specified host instead of generating a new one.";
    };

    nginx = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = "Additional nginx configuration to merge with the default virtual host config.";
    };
  };

  config = lib.mkIf config.www.enable {
    systemd.services.www = {
      description = "www service";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        User = "nobody";
        Group = "nobody";
        WorkingDirectory = "${www_pkg}/";
        Environment = [
          "REF=refs/heads/main"
          "COMMIT=${builtins.substring 0 8 mono.sources.www.revision}"
          "BIND=${config.www.bindAddr}:${toString config.www.port}"
        ];
        ExecStart = "${www_pkg}/bin/www";
        Restart = "always";
      };
    };

    services.nginx.enable = true;
    services.nginx.virtualHosts = {
      "${config.www.domain}" = lib.mkMerge [
        {
          forceSSL = true;
          enableACME = config.www.useACMEHost == null;
          useACMEHost = config.www.useACMEHost;
          http2 = true;
          http3 = true;
          quic = true;
          extraConfig = ''
            add_header Alt-Svc 'h3=":443"; ma=86400';
          '';
          locations."/" = {
            proxyPass = "http://127.0.0.1:${toString config.www.port}/";
            extraConfig = ''
              proxy_set_header Host $host;
              proxy_set_header X-Real-IP $remote_addr;
              proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
              proxy_set_header X-Forwarded-Proto https;
            '';
          };
        }
        config.www.nginx
      ];
    };
  };
}
