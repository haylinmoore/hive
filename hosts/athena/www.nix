{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
let
  bindUri = "127.0.0.1:15641";
  proxyUri = "http://${bindUri}/";
in
{
  services.derivations.www = hive.web.www.service {
    domain = "hayl.in";
    inherit bindUri;
  };

  proxySites.www = {
    domain = "hayl.in";
    inherit proxyUri;
    useACMEHost = "hayl.in";
  };

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    inherit proxyUri;
    useACMEHost = "hayl.in";
  };
}
