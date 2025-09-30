{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

{
  staticSites.lambda = {
    domain = "lambda.hayl.in";
    source = hive.sources.sheepda;
    useACMEHost = "hayl.in";
  };
}
