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
  staticSites.lambda = {
    domain = "lambda.hayl.in";
    source = sources.sheepda;
    useACMEHost = "hayl.in";
  };
}
