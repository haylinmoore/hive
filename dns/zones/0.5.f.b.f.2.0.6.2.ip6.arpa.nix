{ dns }:
with dns;
{
  registrar = "none";
  provider = "athena";
  ttl = 3600;
  records = [
    (A "@" "1.1.1.1")
  ];
}
