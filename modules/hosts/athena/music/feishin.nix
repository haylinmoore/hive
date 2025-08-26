{
  config,
  lib,
  pkgs,
  ...
}:

{
  virtualisation.oci-containers.containers.feishin = {
    image = "ghcr.io/jeffvli/feishin:latest";
    environment = {
      SERVER_NAME = "music";
      SERVER_TYPE = "jellyfin";
      SERVER_URL = "https://music.hayl.in";
      PUBLIC_PATH = "/feishin";
    };
    ports = [ "127.0.0.1:9180:9180" ];
    autoStart = true;
  };

  services.nginx.virtualHosts."music.hayl.in".locations."/feishin/" = {
    proxyPass = "http://127.0.0.1:9180/feishin/";
    proxyWebsockets = true;
  };
}
