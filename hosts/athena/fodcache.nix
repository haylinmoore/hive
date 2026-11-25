{
  config,
  hive,
  ...
}:
let
  bind = "127.0.0.1:15650";
  stateDir = "/var/lib/fodcache";
  maxSize = 32 * 1024 * 1024 * 1024;
in
{
  environment.persistence."/persistent".directories = [
    {
      directory = stateDir;
      user = "fodcache";
      group = "fodcache";
      mode = "0711";
    }
  ];

  users.users.fodcache = {
    isSystemUser = true;
    group = "fodcache";
    home = stateDir;
  };
  users.groups.fodcache = { };

  services.derivations.fodcache = hive.web.fodcache.service {
    inherit bind stateDir maxSize;
  };

  systemd.services.derivations-fodcache = {
    path = [ config.nix.package ];
    serviceConfig = {
      MemoryMax = "4G";
      ProtectSystem = "strict";
      StateDirectory = "fodcache";
      StateDirectoryMode = "0711";
      PrivateTmp = true;
      NoNewPrivileges = true;
    };
  };

  services.nginx.virtualHosts."fod.hayl.in" = {
    forceSSL = true;
    useACMEHost = "hayl.in";
    root = "${stateDir}/cache";
    extraConfig = ''
      add_header Alt-Svc 'clear' always; # nginx2root: h3 off, see nixos/shared/mitigations.nix
    '';
    locations = {
      "= /".proxyPass = "http://${bind}/";
      "= /upload" = {
        proxyPass = "http://${bind}";
        extraConfig = ''
          client_max_body_size ${toString maxSize};
          proxy_request_buffering off;
          proxy_read_timeout 900s;
          proxy_send_timeout 900s;
          proxy_set_header X-Real-IP $remote_addr;
        '';
      };
      "~ \\.narinfo$".extraConfig = ''
        default_type text/x-nix-narinfo;
      '';
    };
  };
}
