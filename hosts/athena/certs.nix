{
  config,
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

  watching = [
    "qumu.lol"
    "qumulo.com"
    "jordane.day"
    "pussy.army"
    "ezri.pet"
    "ezrizhu.com"
    "starry.cafe"
    "andrewsun.com"
    "asun.co"
    "holliger.me"
    "natesales.net"
    "packetframe.com"
    "as34553.net"
    "arix.dev"
    "libdb.so"
    "lifeless.space"
    "winter.cafe"
    "bunbun.dev"
    "giannirosato.com"
    "varun.ch"
    "pineco.net"
    "pixilic.com"
    "notnite.com"
    "enjarai.dev"
    "umbresp.cat"
    "lily.pet"
    "thomasricci.dev"
    "goldenstack.net"
    "kibty.town"
    "matdoes.dev"
    "slonk.ing"
    "zptr.cc"
    "lina.sh"
    "cuiiliste.de"
    "damcraft.de"
    "miaow.ing"
    "vidbuchanan.co.uk"
    "samranda.com"
    "ptnote.dev"
    "lyra.horse"
    "d14.pet"
    "devhack.net"
    "devhack2.net"
    "yui.dev"
    "exampleusercontent.com"
  ];

  certs = hive.web.certs;
in
{
  services.certspotter = {
    enable = true;
    startAtEnd = true;
    watchlist = map (z: ".${z}") (zones ++ watching);
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

  services.derivations.certs = certs.service {
    inherit bind dir;
    hiveZones = zones;
  };

  sops.secrets.certs = {
    sopsFile = ../../secrets/athena/certs.env;
    key = "";
    format = "dotenv";
    owner = "certspotter";
  };

  systemd.services.certs-backfill = {
    description = "Backfill certs.hayl.in from CT search indexes";
    restartIfChanged = false;
    after = [
      "network-online.target"
      "certspotter.service"
    ];
    wants = [ "network-online.target" ];
    environment = {
      CERTS_DIR = dir;
      CERTS_ZONES = lib.concatStringsSep "," (zones ++ watching);
    };
    serviceConfig = {
      Type = "oneshot";
      User = "certspotter";
      Group = "certspotter";
      ExecStart = "${certs}/bin/certs backfill";
      EnvironmentFile = config.sops.secrets.certs.path;
    };
  };

  systemd.timers.certs-backfill = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnActiveSec = "2min";
      OnUnitInactiveSec = "1h";
    };
  };

  proxySites.certs = {
    domain = "certs.hayl.in";
    proxyUri = "http://${bind}/";
    useACMEHost = "hayl.in";
  };
}
