{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 60;
  records = [
    (A "ns.dns" "207.167.121.65")
    (AAAA "ns.dns" "2602:fbf5:2::1")
    (NS "d.dns" "ns.dns.hayl.in.")
    (A "luna.iad" "207.174.105.129")
    (A "ipmi.luna.iad" "192.168.98.82")
    (CNAME "hived" "haylinmoore.github.io.")
  ];
}
