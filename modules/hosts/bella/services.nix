{ config, lib, pkgs, ... }:

let
  sources = import ../../../npins;
in

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

  staticSites = {
    lambda = {
      domain = "lambda.hayl.in";
      source = sources.sheepda;
    };
    _88x31 = {
      domain = "88x31.hayl.in";
      source = sources._88x31;
    };
  };
}
