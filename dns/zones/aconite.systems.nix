{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records = migadu "aconite.systems" "ixapxmzi" ++ [
    (TXT "@" "hola mundo")
  ];
}
