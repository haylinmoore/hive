{
  config,
  lib,
  ...
}:

let
  host = config.networking.hostName;
  domain = "${host}.infra.hayl.in";
in
{
  security.acme.certs.${domain} = {
    inherit domain;
    extraDomainNames = [ "*.${domain}" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    environmentFile = "/run/secrets/dns";
  };

  defaultHttp.acmeHosts = [ domain ];

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
