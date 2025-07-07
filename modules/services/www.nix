{
  config,
  lib,
  pkgs,
  ...
}:

{
  www = {
    enable = true;
    domain = "hayl.in";
    port = 15641;
  };

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    proxyUri = "http://localhost:15641/";
    ssl = false;
  };
}
