{ dns }:
with dns;
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 3600;
  records = migadu "haylinmoore.com" "zqnp5c5z" ++ [
    (RDR "@" "https://hayl.in")
    (TXT "@" "google-site-verification=Ld2Bykdp2UFVLt2TJJqltPWNa4p0wsW8JuT42L1K0Y8")
    (TXT "@" "ring-prev:https://ezri.pet/,ring-next:https://ezri.pet/")
  ];
}
