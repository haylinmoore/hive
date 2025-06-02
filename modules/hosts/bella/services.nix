{ config, lib, pkgs, ... }:

let
  sources = import ../../../npins;
in

{
  services.mysql = {
    enable = true;
    package = pkgs.mariadb;
  };

  services.soft-serve = {
    enable = true;
    settings = {
      name = "haylin's repos";
      log_format = "text";
      ssh = {
        listen_addr = ":2222";
        public_url = "ssh://soft.hayl.in:2222";
        max_timeout = 30;
        idle_timeout = 120;
      };
      initial_admin_keys = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL me@haylinmoore.com" ];
    };
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

  www = {
    enable = true;
    domain = "hayl.in";
    port = 15641;
  };

}
