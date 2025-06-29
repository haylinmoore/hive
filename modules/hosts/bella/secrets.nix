{ config, ... }:
{
  sops.secrets."dollpublish" = {
    sopsFile = ../../../secrets/bella/dollpublish.json;
    key = "";
    format = "json";
    owner = config.systemd.services.dollpublish.serviceConfig.User;
    path = "/home/dollpublish/users.json";
    restartUnits = [ "dollpublish.service" ];
  };

  sops.secrets."slskd" = {
    sopsFile = ../../../secrets/bella/slskd.env;
    key = "";
    format = "dotenv";
    owner = config.systemd.services.slskd.serviceConfig.User;
    restartUnits = [ "slskd.service" ];
  };
}
