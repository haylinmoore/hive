{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."hayl.in" = {
    domain = "hayl.in";
    extraDomainNames = [ "*.hayl.in" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    credentialsFile = "/run/secrets/dns";
  };

  defaultHttp.acmeHosts = [ "hayl.in" ];

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
