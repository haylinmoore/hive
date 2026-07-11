{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
let
  dialUri = "127.0.0.1:15642";
  panelUri = "127.0.0.1:15643";
  dbPath = "/var/lib/dial/";
in
{
  environment.persistence."/persistent".directories = [
    dbPath
  ];
  users.users.dial-service = {
    isSystemUser = true;
    group = "dial-service";
    home = dbPath;
  };

  users.groups.dial-service = { };

  systemd.tmpfiles.rules = [
    "d ${dbPath} 0775 dial-service dial-service - -"
  ];

  services.derivations.dial = hive.web.dial.service {
    domain = "dial.uwu.estate";
    inherit dialUri panelUri dbPath;
  };

  proxySites.dial = {
    domain = "dial.uwu.estate";
    proxyUri = "http://${dialUri}/";
    useACMEHost = "uwu.estate";
  };

  services.nginx.virtualHosts."fun.hayl.in".locations."/dial/" = {
    proxyPass = "http://${panelUri}/";
    extraConfig = hive.web.authentik;
  };
}
