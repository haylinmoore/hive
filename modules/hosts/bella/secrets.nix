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
}
