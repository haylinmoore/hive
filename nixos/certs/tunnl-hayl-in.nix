{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."tunnl.hayl.in" = {
    domain = "tunnl.hayl.in";
    extraDomainNames = [ "*.tunnl.hayl.in" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    environmentFile = "/run/secrets/dns";
  };
}
