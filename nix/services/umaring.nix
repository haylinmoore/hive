{
  config,
  lib,
  pkgs,
  ...
}:

let
  sources = import ../../npins;
  umaring = import sources.umaring { inherit pkgs; };
in
{
  systemd.services.umaring = {
    description = "Umaring service";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];

    environment = {
      BIND = "127.0.0.1:3432";
    };

    serviceConfig = {
      Type = "simple";
      ExecStart = "${umaring}/bin/umaring";
      Restart = "always";
      RestartSec = 5;

      # Security hardening
      DynamicUser = true;
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = [
        "AF_INET"
        "AF_INET6"
      ];
      RestrictNamespaces = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      SystemCallFilter = "@system-service";
      SystemCallErrorNumber = "EPERM";
    };
  };

  # Nginx configuration for umaring.mkr.cx
  services.nginx = {
    enable = true;
    virtualHosts."umaring.mkr.cx" = {
      forceSSL = true;
      enableACME = true;
      locations."/" = {
        proxyPass = "http://127.0.0.1:3432";
        proxyWebsockets = true;
        extraConfig = ''
          proxy_set_header Host $host;
          proxy_set_header X-Real-IP $remote_addr;
          proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
          proxy_set_header X-Forwarded-Proto $scheme;
        '';
      };
    };
  };
}
