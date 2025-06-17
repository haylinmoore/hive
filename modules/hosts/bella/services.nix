{
  config,
  lib,
  pkgs,
  ...
}:

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
      http = {
        listen_addr = "127.0.0.1:15642";
        public_url = "https://soft.hayl.in";
      };
      initial_admin_keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL me@haylinmoore.com"
      ];
    };
  };

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

  proxySites = {
    soft = {
      domain = "soft.hayl.in";
      proxyUri = "http://127.0.0.1:15642/";
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
    _256 = {
      domain = "256.121.167.207.in-addr.arpa";
      source = sources._256;
      ssl = false;
    };
  };

  www = {
    enable = true;
    domain = "hayl.in";
    port = 15641;
  };

  services.yggdrasil = {
    enable = true;
    persistentKeys = true;
    settings = {
      Peers = [
        "tcp://longseason.1200bps.xyz:13121"
        "tcp://ygg-pa.incognet.io:8883"
        "tcp://ygg-kcmo.incognet.io:8883"
      ];
    };
  };

}
