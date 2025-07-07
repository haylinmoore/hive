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
  staticSites._88x31 = {
    domain = "88x31.hayl.in";
    source = sources._88x31;
  };
}
