{
  config,
  lib,
  pkgs,
  ...
}:

{
  environment.persistence."/persistent".directories = [
    "/var/lib/jellyfin"
    "/var/cache/jellyfin"
  ];

  services.jellyfin = {
    enable = true;
    user = "jellyfin";
    group = "media";
    openFirewall = false;
  };

  users.users.jellyfin.extraGroups = [ "media" ];

  proxySites.jellyfin = {
    domain = "jellyfin.hayl.in";
    proxyUri = "http://127.0.0.1:8096";
    useACMEHost = "hayl.in";
  };
}
