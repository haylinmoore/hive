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
    credentialsFile = "/run/secrets/dns";
  };

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}