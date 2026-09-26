{ ... }:
{
  # Services
  services.openssh.enable = true;
  services.upower.enable = true;
  services.printing.enable = true;
  services.lldpd.enable = true;
  services.tailscale.enable = true;
  services.strongswan.enable = true;

  # Yggdrasil
  services.yggdrasil = {
    enable = true;
    persistentKeys = true;
    settings = {
      Peers = [
        "tcp://longseason.1200bps.xyz:13121"
        "tcp://ygg-pa.incognet.io:8883"
        "tcp://ygg-kcmo.incognet.io:8883"
      ];
    };
  };
}
