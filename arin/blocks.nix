{ arin }:
with arin;
{
  "23.144.156.0/24" = {
    origin = 923;
    route = [ "23.144.156.0/24" ];
    rpki = [ "23.144.156.0/24" ];
    rdns."156.144.23.in-addr.arpa" = athena;
  };

  "131.143.204.0/22" = {
    origin = 923;
    route = [ "131.143.204.0/22" ];
    rpki = [
      {
        prefix = "131.143.204.0/22";
        maxLength = 24;
      }
    ];
    rdns."204.143.131.in-addr.arpa" = athena;
    rdns."205.143.131.in-addr.arpa" = athena;
    rdns."206.143.131.in-addr.arpa" = athena;
    rdns."207.143.131.in-addr.arpa" = athena;
  };

  "2602:fbf5::/36" = {
    origin = 923;
    route6 = [
      "2602:fbf5::/36"
      "2602:fbf5::/48"
      "2602:fbf5:3::/48"
    ];
    rpki = [
      {
        prefix = "2602:fbf5::/36";
        maxLength = 48;
      }
    ];
    rdns."0.5.f.b.f.2.0.6.2.ip6.arpa" = athena;
  };

  "207.167.121.0/24" = {
    origin = 923;
    route = [ "207.167.121.0/24" ];
    rpki = [ "207.167.121.0/24" ];
    rdns."121.167.207.in-addr.arpa" = athena;
  };
}
