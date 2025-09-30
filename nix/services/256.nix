{
  config,
  lib,
  pkgs,
  mono,
  ...
}:

{
  staticSites._256 = {
    domain = "256.121.167.207.in-addr.arpa";
    source = mono.sources._256;
    ssl = false;
  };
}
