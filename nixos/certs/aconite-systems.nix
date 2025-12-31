{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."aconite.systems" = {
    domain = "aconite.systems";
    extraDomainNames = [ "*.aconite.systems" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    credentialsFile = "/run/secrets/dns";
  };

  defaultHttp.acmeHosts = [ "aconite.systems" ];

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
