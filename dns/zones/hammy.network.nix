{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records =
    migadu "hammy.network" "gbckqiqx"
    ++ dual "@" "207.167.121.64" "2602:fbf5:2::"
    ++ [
      (CNAME "status" "nywo.status.updown.io.")
      (A "luna.iad" "207.174.105.129")
      (A "ipmi.luna.iad" "192.168.98.82")
      (A "vm1.iad" "207.167.121.2")
      (A "k8s.iad" "207.167.121.5")
      (A "ingest.iad" "207.167.121.3")
      (AAAA "k8s.iad" "2602:fbf5:1::5")
      (A "*.k8s.iad" "207.167.121.5")
      (A "plex" "207.167.121.14")
    ]
    ++ dual "k8s.yyz1" "207.167.121.64" "2602:fbf5:2::"
    ++ dual "*.k8s.yyz1" "207.167.121.64" "2602:fbf5:2::";
}
