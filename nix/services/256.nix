{
  config,
  lib,
  pkgs,
  ...
}:

let
  sources = import ../../npins;
in

{
  staticSites._256 = {
    domain = "256.121.167.207.in-addr.arpa";
    source = sources._256;
    ssl = false;
  };
}
