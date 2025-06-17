{
  config,
  lib,
  pkgs,
  ...
}:

let
  sources = import ../../npins;
  www_pkg = import sources.www { inherit pkgs; };
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
          "COMMIT=${builtins.substring 0 8 sources.www.revision}"
          "BIND=${config.www.bindAddr}:${toString config.www.port}"
        ];
        ExecStart = "${www_pkg}/bin/www";
        Restart = "always";
      };
    };

    services.nginx.enable = true;
    services.nginx.virtualHosts = {
      "${config.www.domain}" = {
        forceSSL = true;
        enableACME = true;
        locations."/" = {
          proxyPass = "http://127.0.0.1:${toString config.www.port}/";
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
}
