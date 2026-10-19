{ dns }:
with dns;
{
  registrar = "none";
  provider = "athena";
  ttl = 3600;
  records = [
    (A "@" "1.1.1.1")
    (NS "b.0.0.b.0.0.0" "eyes.ns.xny.onl.")
    (NS "b.0.0.b.0.0.0" "cooing.ns.xny.onl.")
  ];
}
