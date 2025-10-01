{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

{
  staticSites._256 = {
    domain = "256.121.167.207.in-addr.arpa";
    source = hive.web."256";
    ssl = false;
  };
}
