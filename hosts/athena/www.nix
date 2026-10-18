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

  proxySites.www-http = {
    domain = "http.hayl.in";
    ssl = false;
    inherit proxyUri;
  };

  dns.records."v6.hayl.in".AAAA = config.dns.addresses.v6;
  dns.records."ygg.hayl.in".AAAA = "204:710b:f568:ec5e:1769:f461:9498:cb6d";

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    inherit proxyUri;
    useACMEHost = "hayl.in";
  };
}
