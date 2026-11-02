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
    source = hive.web.lambda;
    useACMEHost = "hayl.in";
  };
}
