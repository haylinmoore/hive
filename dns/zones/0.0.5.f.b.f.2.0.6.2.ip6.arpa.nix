{ dns }:
with dns;
{
  registrar = "none";
  provider = "desec";
  ttl = 3600;
  records = migadu "0.0.5.f.b.f.2.0.6.2.ip6.arpa" "fm6gowfy" ++ [
    (A "@" "1.1.1.1")
    (PTR "2602:fbf5:1::1" "luna.iad.hammy.network.")
    (PTR "2602:fbf5:1::5" "k8s.iad.hammy.network.")
    (PTR "2602:fbf5:2::" "k8s-lb-v6.iad.hammy.network.")
    (NS "b.0.0.b.0.0" "eyes.ns.xny.onl.")
    (NS "b.0.0.b.0.0" "cooing.ns.xny.onl.")
  ];
}
