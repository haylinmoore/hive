{ ... }:
{
  networking = {
    hostName = "zoe";
    useDHCP = true;
    dhcpcd.enable = true;
  };

  boot.kernel.sysctl = {
    "net.ipv4.conf.all.forwarding" = 1;
    "net.ipv6.conf.all.forwarding" = 1;
  };
}
