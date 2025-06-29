{ config, ... }:
{
  sops.secrets."duckdns" = {
    sopsFile = ../../../secrets/zoe/tokens.yaml;
    key = "duckdns";
    owner = config.systemd.services.duckdns-ds.serviceConfig.User;
    restartUnits = [ "duckdns-ds.service" ];
  };
}
