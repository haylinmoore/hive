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

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
