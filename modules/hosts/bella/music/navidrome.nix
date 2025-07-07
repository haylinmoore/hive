{
  config,
  lib,
  pkgs,
  ...
}:

rec {
  sops.secrets."navidrome" = {
    sopsFile = ../../../../secrets/bella/navidrome.env;
    key = "";
    format = "dotenv";
    owner = config.systemd.services.navidrome.serviceConfig.User;
    restartUnits = [ "navidrome.service" ];
  };

  services.navidrome = {
    enable = true;
    user = "navidrome";
    group = "media";
    environmentFile = "/run/secrets/navidrome";
    settings = {
      Port = 4533;
      Address = "127.0.0.1";
      BaseUrl = "/navi";
      MusicFolder = "/music";
    };
  };

  services.nginx.virtualHosts."music.hayl.in".locations."/navi/" = {
    proxyPass = "http://127.0.0.1:${toString services.navidrome.settings.Port}/navi/";
    proxyWebsockets = true;
  };
}
