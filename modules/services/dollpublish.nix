{
  config,
  lib,
  pkgs,
  ...
}:

{
  dollpublish = {
    enable = true;
    dataDir = "/home/dollpublish";
    port = 15640;
    domain = "estrogen.coffee";
    useACMEHost = "estrogen.coffee";
    domainAliases = {
      "share.hayl.in" = {
        username = "haylin";
        useACMEHost = "hayl.in";
      };
      "qshare.hayl.in" = {
        username = "qaylin";
        useACMEHost = "hayl.in";
      };
    };
  };

  sops.secrets."dollpublish" = {
    sopsFile = ../../secrets/bella/dollpublish.json;
    key = "";
    format = "json";
    owner = config.systemd.services.dollpublish.serviceConfig.User;
    path = "/home/dollpublish/users.json";
    restartUnits = [ "dollpublish.service" ];
  };
}
