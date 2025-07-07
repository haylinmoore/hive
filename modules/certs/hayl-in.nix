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
    dnsProvider = "desec";
    dnsPropagationCheck = true;
    credentialsFile = "/run/secrets/dns";
  };

  users.users.nginx.extraGroups = [ "acme" ];
}
