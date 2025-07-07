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
  };

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    proxyUri = "http://localhost:${toString www.port}/";
    ssl = false;
  };
}
