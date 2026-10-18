{ dns }:
with dns;
{
  registrar = "none";
  provider = "bunny";
  ttl = 3600;
  records = [
    (ALIAS "@" "haylinmoore.github.io.")
  ];
}
