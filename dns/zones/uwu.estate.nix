{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 1;
  records = [
    (AAAA "skye" "fd8a:ba9a:aa04::1")
  ];
}
