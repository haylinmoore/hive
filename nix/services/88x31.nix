{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

{
  staticSites._88x31 = {
    domain = "88x31.hayl.in";
    source = hive.web."88x31";
    useACMEHost = "hayl.in";
  };
}
