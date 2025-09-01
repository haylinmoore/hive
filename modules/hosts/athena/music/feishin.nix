{
  config,
  lib,
  pkgs,
  ...
}:

{
  virtualisation.oci-containers.containers.feishin = {
    image = "ghcr.io/jeffvli/feishin:0.19.0";
    environment = {
      SERVER_NAME = "music";
      SERVER_TYPE = "navidrome";
      SERVER_LOCK = "true";
      SERVER_URL = "https://music.hayl.in/navi";
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
