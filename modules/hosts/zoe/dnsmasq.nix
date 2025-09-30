{
  config,
  lib,
  pkgs,
  ...
}:
{
  services.dnsmasq = {
    enable = true;
    resolveLocalQueries = false;
    settings = {
      # Listen on all interfaces except loopback
      bind-dynamic = true;
      except-interface = "lo";

      # Don't read /etc/resolv.conf for upstream servers
      no-resolv = true;

      # Act as authoritative server for these zones, forwarding to router
      server = [
        "/lan.uwu.estate/192.168.42.1"
        "/42.168.192.in-addr.arpa/192.168.42.1"
      ];

      # Don't provide DHCP
      no-dhcp-interface = "";
    };
  };

  networking.firewall.allowedUDPPorts = [ 53 ];
  networking.firewall.allowedTCPPorts = [ 53 ];
}
