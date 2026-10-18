// Records are generated from Nix, see ./default.nix and `./upgrade.sh dns`.
var REGISTRARS = {
  porkbun: NewRegistrar("porkbun"),
  none: NewRegistrar("none"),
};

var PROVIDERS = {
  bunny: NewDnsProvider("bunny_dns"),
  desec: NewDnsProvider("desec"),
};

var TYPES = {
  A: A,
  AAAA: AAAA,
  ALIAS: ALIAS,
  BUNNY_DNS_RDR: BUNNY_DNS_RDR,
  CNAME: CNAME,
  IGNORE: IGNORE,
  MX: MX,
  NS: NS,
  PTR: PTR,
  SRV: SRV,
  TXT: TXT,
};

var zones = require("./records.json").zones;

for (var name in zones) {
  var zone = zones[name];
  D(
    name,
    REGISTRARS[zone.registrar],
    DnsProvider(PROVIDERS[zone.provider]),
    DefaultTTL(zone.ttl),
    zone.dnssec ? AUTODNSSEC_ON : [],
    zone.records.map(function (r) {
      return TYPES[r.type].apply(null, r.args);
    }),
  );
}
