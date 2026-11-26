{
  config,
  lib,
  pkgs,
  ...
}:

{
  security.acme.certs."fods.lol" = {
    domain = "fods.lol";
    extraDomainNames = [ "*.fods.lol" ];
    dnsProvider = "bunny";
    dnsPropagationCheck = true;
    environmentFile = "/run/secrets/dns";
  };

  defaultHttp.acmeHosts = [ "fods.lol" ];

  users.users = lib.mkIf config.services.nginx.enable {
    nginx.extraGroups = [ "acme" ];
  };
}
