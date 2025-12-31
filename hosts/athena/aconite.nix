{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

{
  staticSites.aconite = {
    domain = "aconite.systems";
    source = hive.web.aconite;
    useACMEHost = "aconite.systems";
  };
}
