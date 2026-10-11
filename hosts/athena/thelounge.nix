{
  config,
  lib,
  pkgs,
  ...
}:

{
  environment.persistence."/persistent".directories = [
    "/var/lib/thelounge"
  ];

  services.thelounge = {
    enable = true;
    port = 9005;
    public = false;
  };

  # crashes if the startup changelog fetch fails, which it does until bgp converges
  systemd.services.thelounge.serviceConfig = {
    Restart = "on-failure";
    RestartSec = 10;
  };

  services.nginx.virtualHosts."irc.hayl.in" = {
    forceSSL = true;
    useACMEHost = "hayl.in";
    locations."/" = {
      proxyPass = "http://127.0.0.1:9005";
      proxyWebsockets = true;
    };
  };
}
