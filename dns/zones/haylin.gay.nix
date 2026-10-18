{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records = [
    (RDR "@" "https://hayl.in")
    (TXT "_atproto" "did=did:plc:az4x5wmcomdybtrjgig4nvjn")
  ];
}
