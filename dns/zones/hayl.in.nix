{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 60;
  records = [
    (CNAME "hived" "haylinmoore.github.io.")
  ];
}
