{
  lib,
  pkgs,
  hive,
  ...
}:

let
  bind = "127.0.0.1:15700";
  dir = "/var/lib/certspotter/records";

  zones = lib.filter (z: !lib.hasSuffix ".arpa" z) (
    map (lib.removeSuffix ".nix") (lib.attrNames (builtins.readDir ../../dns/zones))
  );

  certs = hive.web.certs;
in
{
  services.certspotter = {
    enable = true;
    startAtEnd = true;
    watchlist = map (z: ".${z}") zones;
    hooks = [
      (pkgs.writeShellScript "certs-record" ''
        exec ${certs}/bin/certs record
      '')
    ];
  };
  systemd.services.certspotter.environment.CERTS_DIR = dir;

  persist.directories = [
    {
      directory = "/var/lib/certspotter";
      user = "certspotter";
      group = "certspotter";
      mode = "0755";
    }
  ];

  users.users.certs = {
    isSystemUser = true;
    group = "certs";
  };
  users.groups.certs = { };

  services.derivations.certs = certs.service { inherit bind dir zones; };

  proxySites.certs = {
    domain = "certs.hayl.in";
    proxyUri = "http://${bind}/";
    useACMEHost = "hayl.in";
  };
}
