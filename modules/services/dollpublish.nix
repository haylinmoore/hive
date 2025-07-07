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
    domainAliases = {
      "share.hayl.in" = "haylin";
      "qshare.hayl.in" = "qaylin";
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
