{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

let
  v4Prefix = "23.144.156.224/27";
  v6Prefix = "2602:fbf5:3:c7::/64";
  apex4 = "23.144.156.224";
  apex6 = "2602:fbf5:3:c7::";

  hostAddress = "fd00:2::1";
  localAddress = "fd00:2::2";
  webPort = 15710;
  tproxyPort = 61999;

  # Same reasoning as tunnl: the state dir is bind mounted, so the owner has
  # to be the same number on both sides.
  id = 975;

  stateDir = "/var/lib/honeypot";
  keyPath = config.sops.secrets.honeypot.path;

  honeypot = hive.web.honeypot;

  # Our own outgoing connections (ACME, resolvers) come back to addresses in
  # the captured prefixes, so replies must skip TPROXY.
  rules = pkgs.writeText "honeypot.nft" ''
    table inet honeypot {
      chain prerouting {
        type filter hook prerouting priority mangle; policy accept;
        ct direction reply accept
        meta l4proto { tcp, udp } th dport 53 accept
        ip daddr ${v4Prefix} meta l4proto { tcp, udp } tproxy ip to :${toString tproxyPort} meta mark set 1 accept
        ip6 daddr ${v6Prefix} meta l4proto { tcp, udp } tproxy ip6 to :${toString tproxyPort} meta mark set 1 accept
      }
    }
  '';
in
{
  users.users.honeypot = {
    isSystemUser = true;
    uid = id;
    group = "honeypot";
    home = stateDir;
    createHome = false;
  };
  users.groups.honeypot.gid = id;

  persist.directories = [
    {
      directory = stateDir;
      user = "honeypot";
      group = "honeypot";
      mode = "0750";
    }
  ];

  sops.secrets.honeypot = {
    sopsFile = ../../secrets/athena/honeypot.yaml;
    key = "key";
    owner = "honeypot";
    restartUnits = [ "container@honeypot.service" ];
  };

  boot.kernelModules = [
    "nft_tproxy"
    "nf_tproxy_ipv4"
    "nf_tproxy_ipv6"
  ];

  containers.honeypot = {
    ephemeral = true;
    autoStart = true;

    privateNetwork = true;
    hostAddress6 = hostAddress;
    localAddress6 = localAddress;

    bindMounts = {
      ${stateDir} = {
        hostPath = stateDir;
        isReadOnly = false;
      };
      ${keyPath} = {
        hostPath = keyPath;
        isReadOnly = true;
      };
    };

    config =
      { pkgs, ... }:
      {
        system.stateVersion = "25.11";

        users.users.honeypot = {
          isSystemUser = true;
          uid = id;
          group = "honeypot";
          home = stateDir;
          createHome = false;
        };
        users.groups.honeypot.gid = id;

        networking = {
          firewall.enable = false;
          interfaces.eth0.ipv6.addresses = [
            {
              address = localAddress;
              prefixLength = 64;
            }
          ];
          interfaces.lo = {
            ipv4.addresses = [
              {
                address = apex4;
                prefixLength = 32;
              }
            ];
            ipv6.addresses = [
              {
                address = apex6;
                prefixLength = 128;
              }
            ];
          };
          nameservers = [
            "2606:4700:4700::1111"
            "2620:fe::fe"
          ];
        };

        # Every address in both prefixes is local, TPROXY hands non-DNS ports
        # to the listener, and outgoing traffic leaves from the apex addresses.
        systemd.services.honeypot-net = {
          after = [ "network-addresses-eth0.service" ];
          wants = [ "network-addresses-eth0.service" ];
          before = [ "honeypot.service" ];
          wantedBy = [ "multi-user.target" ];
          path = [
            pkgs.iproute2
            pkgs.nftables
          ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
          };
          script = ''
            ip route replace local ${v4Prefix} dev lo
            ip -6 route replace local ${v6Prefix} dev lo
            ip rule del fwmark 1 lookup 100 2>/dev/null || true
            ip rule add fwmark 1 lookup 100
            ip route replace local 0.0.0.0/0 dev lo table 100
            ip -6 rule del fwmark 1 lookup 100 2>/dev/null || true
            ip -6 rule add fwmark 1 lookup 100
            ip -6 route replace local ::/0 dev lo table 100
            ip -6 route replace default via ${hostAddress} src ${apex6}
            ip route replace 0.0.0.0/0 via inet6 ${hostAddress} src ${apex4}
            nft delete table inet honeypot 2>/dev/null || true
            nft -f ${rules}
          '';
        };

        systemd.services.honeypot = {
          description = "startguess.ing CT honeypot";
          wantedBy = [ "multi-user.target" ];
          after = [
            "network-online.target"
            "honeypot-net.service"
          ];
          wants = [ "network-online.target" ];
          requires = [ "honeypot-net.service" ];

          environment = {
            HONEYPOT_KEY_FILE = keyPath;
            HONEYPOT_STATE = stateDir;
            HONEYPOT_WEB = "[${localAddress}]:${toString webPort}";
            HONEYPOT_LISTEN_PORT = toString tproxyPort;
            # 2026-10-25, the first day a bait certificate is ordered.
            HONEYPOT_LAUNCH_DAY = "20751";
          };

          serviceConfig = {
            ExecStart = "${honeypot}/bin/honeypot serve";
            Restart = "always";
            RestartSec = 5;

            User = "honeypot";
            Group = "honeypot";
            WorkingDirectory = stateDir;

            LimitNOFILE = 1048576;
            AmbientCapabilities = [
              "CAP_NET_ADMIN"
              "CAP_NET_BIND_SERVICE"
            ];
            CapabilityBoundingSet = [
              "CAP_NET_ADMIN"
              "CAP_NET_BIND_SERVICE"
            ];
            NoNewPrivileges = true;
            ProtectSystem = "strict";
            ProtectHome = true;
            PrivateDevices = true;
            PrivateTmp = true;
            ReadWritePaths = [ stateDir ];
          };
        };
      };
  };

  systemd.services.honeypot-container-route = {
    after = [ "container@honeypot.service" ];
    bindsTo = [ "container@honeypot.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = pkgs.writeShellScript "add-honeypot-routes" ''
        ${pkgs.iproute2}/bin/ip route replace ${v4Prefix} via inet6 ${localAddress}
        ${pkgs.iproute2}/bin/ip -6 route replace ${v6Prefix} via ${localAddress}
      '';
      ExecStop = pkgs.writeShellScript "del-honeypot-routes" ''
        ${pkgs.iproute2}/bin/ip route del ${v4Prefix} via inet6 ${localAddress} || true
        ${pkgs.iproute2}/bin/ip -6 route del ${v6Prefix} via ${localAddress} || true
      '';
    };
  };

  systemd.services.honeypot-ip2asn = {
    description = "Fetch the iptoasn table for the honeypot";
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    path = [ pkgs.curl ];
    serviceConfig = {
      Type = "oneshot";
      User = "honeypot";
      Group = "honeypot";
    };
    script = ''
      curl -fsSL --retry 3 -o ${stateDir}/ip2asn-combined.tsv.gz.tmp \
        https://iptoasn.com/data/ip2asn-combined.tsv.gz
      mv ${stateDir}/ip2asn-combined.tsv.gz.tmp ${stateDir}/ip2asn-combined.tsv.gz
    '';
  };

  systemd.timers.honeypot-ip2asn = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnActiveSec = "2min";
      OnCalendar = "weekly";
      Persistent = true;
    };
  };

  proxySites.honeypot = {
    domain = "honeypot.hayl.in";
    proxyUri = "http://[${localAddress}]:${toString webPort}/";
    useACMEHost = "hayl.in";
  };
}
