{
  config,
  lib,
  pkgs,
  ...
}:

rec {
  environment.persistence."/persistent".directories = [ dollpublish.dataDir ];

  dollpublish = {
    enable = true;
    dataDir = "/var/lib/dollpublish";
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
    sopsFile = ../../secrets/dollpublish.json;
    key = "";
    format = "json";
    owner = config.systemd.services.dollpublish.serviceConfig.User;
    path = "${dollpublish.dataDir}/users.json";
    restartUnits = [ "dollpublish.service" ];
  };
}
