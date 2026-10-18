{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records = migadu "hammy.network" "gbckqiqx";
}
