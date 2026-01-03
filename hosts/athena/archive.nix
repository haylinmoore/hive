{
  config,
  lib,
  pkgs,
  ...
}:

{
  services.nginx.virtualHosts."archive.hayl.in" = {
    forceSSL = true;
    useACMEHost = "hayl.in";
    http2 = true;
    http3 = true;
    quic = true;
    root = "/bulk/archive";
    extraConfig = ''
      add_header Alt-Svc 'h3=":443"; ma=86400';
      autoindex on;
    '';
  };
}
