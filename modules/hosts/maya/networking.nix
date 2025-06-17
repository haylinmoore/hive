{ config, lib, pkgs, ... }:

{
  networking = {
    hostName = "maya";
    interfaces.eth0.ipv4.addresses = [{
      address = "207.167.121.9";
      prefixLength = 26;
    }];
    interfaces.eth0.ipv6.addresses = [{
      address = "2602:fbf5:1::9";
      prefixLength = 48;
    }];
    defaultGateway = {
      address = "207.167.121.1";
      interface = "eth0";
    };
    defaultGateway6 = {
      address = "2602:fbf5:1::1";
      interface = "eth0";
    };
    dhcpcd.enable = false;
  };
}
