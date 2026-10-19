{
  config,
  lib,
  pkgs,
  nodes,
  ...
}:

let
  zonefile = import ../../dns/zonefile.nix { inherit lib; };
  zones =
    lib.filterAttrs (_: zone: zone.provider == "athena")
      (import ../../dns { inherit nodes lib; }).zones;
in
{
  dns.records."ns1.aconite.systems" = config.dns.here;
  dns.records."ns2.aconite.systems" = config.dns.here;

  services.knot = {
    enable = true;
    settings = {
      server.listen = [
        "${config.dns.addresses.v4}@53"
        "${config.dns.addresses.v6}@53"
      ];
      acl = [
        {
          id = "axfr";
          address = [
            "0.0.0.0/0"
            "::/0"
          ];
          action = "transfer";
        }
      ];
      zone = lib.mapAttrsToList (name: zone: {
        domain = name;
        acl = [ "axfr" ];
        file = pkgs.writeText "${name}.zone" (zonefile zone);
        zonefile-sync = -1;
        zonefile-load = "difference-no-serial";
        journal-content = "all";
      }) zones;
    };
  };

  environment.persistence."/persistent".directories = [
    {
      directory = "/var/lib/knot";
      user = "knot";
      group = "knot";
      mode = "0700";
    }
  ];

  networking.firewall.allowedTCPPorts = [ 53 ];
  networking.firewall.allowedUDPPorts = [ 53 ];
}
