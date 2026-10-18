{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records = [
    (CNAME "ha" "9ty9.duckdns.org.")
    (A "alyx" "192.168.42.1")
    (A "zoe" "192.168.42.3")
  ];
}
