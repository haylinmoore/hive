{ ... }:
{
  networking = {
    hostName = "zoe";
    useDHCP = true;
    dhcpcd.enable = true;
  };
}
