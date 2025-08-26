{
  config,
  lib,
  pkgs,
  ...
}:

{
  sops.secrets."slskd" = {
    sopsFile = ../../../../secrets/bella/slskd.env;
    key = "";
    format = "dotenv";
    owner = config.systemd.services.slskd.serviceConfig.User;
    restartUnits = [ "slskd.service" ];
  };

  services.slskd = {
    enable = true;
    user = "slskd";
    openFirewall = true;
    domain = "music.hayl.in";
    environmentFile = "/run/secrets/slskd";
    settings = {
      web.url_base = "/slskd";
      shares.directories = [
        "/music/haylin"
        "/music/alice"
      ];
      directories.downloads = "/music/haylin/unsorted";
    };
  };
}
