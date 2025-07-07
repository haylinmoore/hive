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
    dnsProvider = "porkbun";
    dnsPropagationCheck = true;
    credentialsFile = "/run/secrets/dns";
  };

  users.users.nginx.extraGroups = [ "acme" ];
}
