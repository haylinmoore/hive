{ lib, ... }:
{
  networking = {
    hostName = "zoe";
    useDHCP = true;
    dhcpcd.enable = true;
  };

  boot.kernel.sysctl = {
    "net.ipv4.conf.all.forwarding" = lib.mkForce 1;
    "net.ipv6.conf.all.forwarding" = lib.mkForce 1;
  };
}
