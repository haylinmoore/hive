{ dns }:
with dns;
{
  registrar = "none";
  provider = "desec";
  ttl = 3600;
  records = migadu "0.0.5.f.b.f.2.0.6.2.ip6.arpa" "fm6gowfy" ++ [
    (A "@" "1.1.1.1")
    (NS "b.0.0.b.0.0" "eyes.ns.xny.onl.")
    (NS "b.0.0.b.0.0" "cooing.ns.xny.onl.")
  ];
}
