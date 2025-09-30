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
    source = hive.sources._256;
    ssl = false;
  };
}
