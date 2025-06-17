{ config, lib, pkgs, ... }:

{
  networking = {
    hostName = "bella";
    interfaces.ens18.ipv4.addresses = [{
      address = "207.167.121.8";
      prefixLength = 26;
    }];
    interfaces.ens18.ipv6.addresses = [{
      address = "2602:fbf5:1::8";
      prefixLength = 48;
    }];
    defaultGateway = {
      address = "207.167.121.1";
      interface = "ens18";
    };
    defaultGateway6 = {
      address = "2602:fbf5:1::1";
      interface = "ens18";
    };
    dhcpcd.enable = false;
  };
}
