{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."uwu.estate" = {
    domain = "uwu.estate";
    extraDomainNames = [ "*.uwu.estate" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    environmentFile = "/run/secrets/dns";
  };

  defaultHttp.acmeHosts = [ "uwu.estate" ];

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
