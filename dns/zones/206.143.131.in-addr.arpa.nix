{ dns }:
{
  registrar = "none";
  provider = "bunny";
  ttl = 3600;
  dnssec = true;
  records = [ ];
}
