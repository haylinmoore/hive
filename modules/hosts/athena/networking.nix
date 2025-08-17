{
  config,
  lib,
  pkgs,
  ...
}:

{
  networking.hostName = "athena";

  boot.kernelModules = [ "bonding" ];

  networking.bonds.bond0 = {
    interfaces = [
      "ens1f0np0"
      "ens1f1np1"
    ];
    driverOptions = {
      mode = "802.3ad";
      miimon = "100";
      lacp_rate = "fast";
      xmit_hash_policy = "layer3+4";
    };
  };

  networking.interfaces.bond0 = {
    ipv4.addresses = [
      {
        address = "199.255.18.178";
        prefixLength = 32;
      }
    ];
    ipv6.addresses = [
      {
        address = "2606:7940:32:36::10";
        prefixLength = 120;
      }
    ];
  };

  networking.defaultGateway6 = {
    address = "2606:7940:32:36::1";
    interface = "bond0";
  };

  networking.interfaces.eno1 = {
    ipv4.addresses = [
      {
        address = "203.0.113.2";
        prefixLength = 24;
      }
    ];
  };

  networking.interfaces.eno2.useDHCP = true;

  networking.useDHCP = false;
}
