{ dns }:
with dns;
{
  registrar = "none";
  provider = "bunny";
  ttl = 3600;
  records = [
    (RDR "haylin" "https://hayl.in")
  ];
}
