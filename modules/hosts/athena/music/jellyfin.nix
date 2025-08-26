{
  config,
  lib,
  pkgs,
  ...
}:

{
  services.jellyfin = {
    enable = true;
    openFirewall = true;
    user = "jellyfin";
    group = "media";
  };

  services.nginx.virtualHosts."music.hayl.in".locations."/".tryFiles = "$uri $uri/ @jellyfin";
  services.nginx.virtualHosts."music.hayl.in".locations."@jellyfin" = {
    proxyPass = "http://127.0.0.1:8096";
    proxyWebsockets = true;
  };
}
