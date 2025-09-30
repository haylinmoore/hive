{
  config,
  lib,
  pkgs,
  mono,
  ...
}:

{
  staticSites._88x31 = {
    domain = "88x31.hayl.in";
    source = mono.web."88x31";
    useACMEHost = "hayl.in";
  };
}
