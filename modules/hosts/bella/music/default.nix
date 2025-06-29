{
  config,
  lib,
  pkgs,
  ...
}:

let
  musicLandingPage = pkgs.writeTextFile {
    name = "music-landing-page";
    text = builtins.readFile ./index.html;
    destination = "/index.html";
  };
in
{
  users.users.alice = {
    isNormalUser = true;
    createHome = true;
    group = "media";
    openssh.authorizedKeys.keys = [ ];
  };

  users.groups.media = { };

  services.jellyfin = {
    enable = true;
    openFirewall = true;
    user = "jellyfin";
    group = "media";
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
      shares.directories = [
        "/music/haylin"
        "/music/alice"
      ];
      directories.downloads = "/music/haylin/unsorted";
    };
  };

  virtualisation.oci-containers.containers.feishin = {
    image = "ghcr.io/jeffvli/feishin:latest";
    environment = {
      SERVER_NAME = "music";
      SERVER_LOCK = "true";
      SERVER_TYPE = "jellyfin";
      SERVER_URL = "https://music.hayl.in";
      PUBLIC_PATH = "/feishin";
    };
    ports = [ "127.0.0.1:9180:9180" ];
    autoStart = true;
  };

  services.nginx = {
    enable = true;
    virtualHosts."music.hayl.in" = {
      forceSSL = true;
      enableACME = true;
      locations = {
        "/" = {
          root = musicLandingPage;
          tryFiles = "$uri $uri/ @jellyfin";
        };
        "@jellyfin" = {
          proxyPass = "http://127.0.0.1:8096";
          proxyWebsockets = true;
        };
        "/feishin/" = {
          proxyPass = "http://127.0.0.1:9180/feishin/";
          proxyWebsockets = true;
        };
      };
    };
  };
}
