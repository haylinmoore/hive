{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.navidrome;
  settingsFormat = pkgs.formats.json { };
in
rec {
  sops.secrets."navidrome" = {
    sopsFile = ../../../secrets/navidrome.env;
    key = "";
    format = "dotenv";
    owner = config.systemd.services.navidrome.serviceConfig.User;
    restartUnits = [ "navidrome.service" ];
  };

  environment.persistence."/persistent".directories = [ "/var/lib/navidrome" ];

  services.navidrome = {
    enable = true;
    user = "navidrome";
    group = "media";
    environmentFile = "/run/secrets/navidrome";
    settings = {
      Port = 4533;
      Address = "127.0.0.1";
      BaseUrl = "/navi";
      MusicFolder = "/bulk/music";
      EnableSharing = true;
    };
  };

  systemd.services.navidrome-scan = {
    description = "Navidrome music library scan";
    serviceConfig = {
      Type = "oneshot";
      User = cfg.user;
      Group = cfg.group;
      WorkingDirectory = "/var/lib/navidrome";
      EnvironmentFile = lib.mkIf (cfg.environmentFile != null) [ cfg.environmentFile ];
      ExecStart = "${lib.getExe cfg.package} scan --configfile ${settingsFormat.generate "navidrome.json" cfg.settings}";
    };
  };

  systemd.timers.navidrome-scan = {
    description = "Run navidrome scan every 5 minutes";
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnCalendar = "*:0/5";
      Persistent = true;
    };
  };

  security.sudo.extraRules = [
    {
      groups = [ "media" ];
      commands = [
        {
          command = "/run/current-system/sw/bin/systemctl start navidrome-scan.service";
          options = [ "NOPASSWD" ];
        }
        {
          command = "/run/current-system/sw/bin/systemctl start navidrome-scan";
          options = [ "NOPASSWD" ];
        }
      ];
    }
  ];

  services.nginx.virtualHosts."music.hayl.in".locations."/navi/" = {
    proxyPass = "http://127.0.0.1:${toString services.navidrome.settings.Port}/navi/";
    proxyWebsockets = true;
  };
}
