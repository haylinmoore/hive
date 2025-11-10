{ lib }:

# Takes options and a service derivation, returns a NixOS module
# that adds the service AND configures nginx based on metadata
{
  useACMEHost ? null,
}:
serviceDrv:

{ config, ... }:

with lib;

let
  meta = serviceDrv.meta;
  domain = meta.domain or (throw "wrapVirtualHost: service ${serviceDrv.name} missing meta.domain");
  bindAddr = meta.bindAddr or "127.0.0.1";
  port = meta.port or (throw "wrapVirtualHost: service ${serviceDrv.name} missing meta.port");
in

{
  # Add the service to services.derivations
  services.derivations = [ serviceDrv ];

  # Configure nginx virtual host
  services.nginx.enable = true;
  services.nginx.virtualHosts."${domain}" = {
    forceSSL = true;
    enableACME = useACMEHost == null;
    useACMEHost = useACMEHost;
    http2 = true;
    http3 = true;
    quic = true;
    extraConfig = ''
      add_header Alt-Svc 'h3=":443"; ma=86400';
    '';
    locations."/" = {
      proxyPass = "http://${bindAddr}:${toString port}/";
      extraConfig = ''
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
      '';
    };
  };
}
