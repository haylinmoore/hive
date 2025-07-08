{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."estrogen.coffee" = {
    domain = "estrogen.coffee";
    extraDomainNames = [ "*.estrogen.coffee" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    credentialsFile = "/run/secrets/dns";
  };

  users.users.nginx.extraGroups = [ "acme" ];
}
