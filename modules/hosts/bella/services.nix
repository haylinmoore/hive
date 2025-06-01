{ config, lib, pkgs, ... }:
{
  services.mysql = {
    enable = true;
    package = pkgs.mariadb;
  };

  dollpublish = {
    enable = true;
    dataDir = "/home/dollpublish";
    port = 15640;
    domain = "estrogen.coffee";
    domainAliases = {
      "share.hayl.in" = "haylin";
    };
  };
}
