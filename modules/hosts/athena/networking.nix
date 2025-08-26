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

  networking.interfaces.lo = {
    ipv4.addresses = [
      {
        address = "199.255.18.178";
        prefixLength = 32;
      }
      {
        address = "127.0.0.1";
        prefixLength = 8;
      }
    ];
    ipv6.addresses = [
      {
        address = "::1";
        prefixLength = 128;
      }
      {
        address = "2602:fbf5:3::";
        prefixLength = 128;
      }
      {
        address = "2602:fbf5::";
        prefixLength = 128;
      }
    ];
  };

  networking.interfaces.bond0 = {
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

  networking.useDHCP = false;

  networking.interfaces.eno1 = {
    ipv4.addresses = [
      {
        address = "169.254.69.69";
        prefixLength = 16;
      }
    ];
  };

  # Configure 25Gb NICs
  networking.interfaces.ens1f0np0 = {
    useDHCP = false;
    ethtool = {
      speed = 25000;
      duplex = "full";
      autoneg = false;
      fec = "rs";
    };
  };

  networking.interfaces.ens1f1np1 = {
    useDHCP = false;
    ethtool = {
      speed = 25000;
      duplex = "full";
      autoneg = false;
      fec = "rs";
    };
  };
}
