{ dns }:
with dns;
let
  mc1 = map toString ([ 3 ] ++ dns.lib.range 32 39);
  k8sLb = map (n: {
    name = toString n;
    value = "k8s-lb-${toString (n - 64)}.iad.hammy.network.";
  }) (dns.lib.range 64 127);
in
{
  registrar = "none";
  provider = "desec";
  ttl = 3600;
  records =
    migadu "121.167.207.in-addr.arpa" "tq6m5bvx"
    ++ [
      (A "420" "5.181.108.112")
      (NS "257" "norman.ns.cloudflare.com.")
      (NS "257" "thea.ns.cloudflare.com.")
      (NS "250" "november.lina.sh.")
      (NS "bunbun" "bunbun.dev.")
    ]
    ++ ptrFill (
      {
        "1" = "hv1.iad.hammy.network.";
        "2" = "vpn.iad.hammy.network.";
        "251" = "november.lina.sh.";
        "252" = "hv1.iad.hammy.network.";
      }
      // dns.lib.genAttrs mc1 (_: "mc1.iad.hammy.network.")
      // builtins.listToAttrs k8sLb
    ) "#.121.167.207.static.hammy.network.";
}
