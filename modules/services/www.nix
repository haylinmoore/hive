{
  config,
  lib,
  pkgs,
  ...
}:

rec {
  www = {
    enable = true;
    domain = "hayl.in";
    port = 15641;
    useACMEHost = "hayl.in";
  };

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    proxyUri = "http://localhost:${toString www.port}/";
    useACMEHost = "hayl.in";
  };
}
