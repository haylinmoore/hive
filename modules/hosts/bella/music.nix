{
  config,
  lib,
  pkgs,
  ...
}:
{
  users.groups.media = { };

  services.jellyfin = {
    enable = true;
    openFirewall = true;
    user = "jellyfin";
    group = "media";
  };
  proxySites.music = {
    domain = "music.hayl.in";
    proxyUri = "http://127.0.0.1:8096/";
  };

  services.slskd = {
    enable = true;
    user = "slskd";
    openFirewall = true;
    domain = "music.hayl.in";
    environmentFile = "/run/secrets/slskd";
    nginx = {
      forceSSL = true;
      enableACME = true;
    };
    settings = {
      web.url_base = "/slskd";
      shares.directories = [ "/music" ];
      directories.downloads = "/music/unsorted";
    };
  };
}
