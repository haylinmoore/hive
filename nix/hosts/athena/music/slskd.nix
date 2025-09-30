{
  config,
  lib,
  pkgs,
  ...
}:

{
  sops.secrets."slskd" = {
    sopsFile = ../../../../secrets/slskd.env;
    key = "";
    format = "dotenv";
    owner = config.systemd.services.slskd.serviceConfig.User;
    restartUnits = [ "slskd.service" ];
  };

  services.slskd = {
    enable = true;
    user = "slskd";
    group = "media";
    openFirewall = true;
    domain = "music.hayl.in";
    environmentFile = "/run/secrets/slskd";
    nginx = {
      useACMEHost = "hayl.in";
      forceSSL = true;
    };
    settings = {
      web.url_base = "/slskd";
      shares.directories = [
        "/bulk/music"
      ];
      directories.downloads = "/bulk/music/haylin/unsorted";
    };
  };
}
