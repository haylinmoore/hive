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

  services.nginx.virtualHosts."jellyfin.hayl.in" = {
    forceSSL = true;
    useACMEHost = "hayl.in";
    locations."/" = {
      proxyPass = "http://127.0.0.1:8096";
      proxyWebsockets = true;
    };
  };
}
