{
  config,
  lib,
  pkgs,
  mono,
  ...
}:

{
  staticSites.lambda = {
    domain = "lambda.hayl.in";
    source = mono.sources.sheepda;
    useACMEHost = "hayl.in";
  };
}
