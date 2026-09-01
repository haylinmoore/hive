{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

let
  domain = "tunnl.hayl.in";

  ipv4 = "23.144.156.6";
  ipv6 = "2602:fbf5:3::6";

  hostAddress = "fd00:1::1";
  localAddress = "fd00:1::2";

  certDir = "/var/lib/acme/${domain}";

  authorizedKeys = pkgs.writeText "tunnl-authorized-keys" (
    lib.concatMapStrings (key: key + "\n") config.users.users.root.openssh.authorizedKeys.keys
  );

  tunnl = hive.web.tunnl;
in
{
  imports = [ ../../nixos/certs/tunnl-hayl-in.nix ];

  # The bind mount below needs the certificate directory to exist before the
  # container starts, which is not guaranteed on a first boot.
  systemd.tmpfiles.rules = [
    "d /persistent/containers/tunnl 0700 root root -"
    "d ${certDir} 0750 acme acme -"
  ];

  # LoadCredential copies the certificate at start, so a renewal only reaches
  # tunnl when the container comes back up.
  security.acme.certs.${domain}.reloadServices = [ "container@tunnl.service" ];

  containers.tunnl = {
    ephemeral = true;
    autoStart = true;

    privateNetwork = true;
    hostAddress6 = hostAddress;
    localAddress6 = localAddress;

    bindMounts = {
      ${certDir} = {
        hostPath = certDir;
        isReadOnly = true;
      };
      "/var/lib/tunnl" = {
        hostPath = "/persistent/containers/tunnl";
        isReadOnly = false;
      };
    };

    config =
      { config, pkgs, ... }:
      {
        system.stateVersion = "25.11";

        systemd.services.tunnl = {
          description = "tunnl SSH tunnel server";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];

          # Keep retrying rather than giving up while acme has yet to issue.
          startLimitIntervalSec = 0;

          environment = {
            SSH_ADDR = ":22";
            HTTP_ADDR = ":80";
            HTTPS_ADDR = ":443";
            DOMAIN = domain;
            HOST_KEY_PATH = "/var/lib/tunnl/host_key";
            AUTHORIZED_KEYS = authorizedKeys.outPath;
            TLS_CERT = "%d/cert";
            TLS_KEY = "%d/key";
          };

          serviceConfig = {
            ExecStart = lib.getExe tunnl;
            Restart = "always";
            RestartSec = 5;

            User = "tunnl";
            Group = "tunnl";
            WorkingDirectory = "/var/lib/tunnl";

            # The cert is owned by acme on the host, whose gid means nothing in
            # here; systemd reads it as root and hands it over before dropping.
            LoadCredential = [
              "cert:${certDir}/fullchain.pem"
              "key:${certDir}/key.pem"
            ];

            AmbientCapabilities = [ "CAP_NET_BIND_SERVICE" ];
            CapabilityBoundingSet = [ "CAP_NET_BIND_SERVICE" ];
            NoNewPrivileges = true;
            ProtectSystem = "strict";
            ProtectHome = true;
            PrivateDevices = true;
            PrivateTmp = true;
            ReadWritePaths = [ "/var/lib/tunnl" ];
          };
        };

        users.users.tunnl = {
          isSystemUser = true;
          group = "tunnl";
          home = "/var/lib/tunnl";
        };
        users.groups.tunnl = { };

        networking = {
          interfaces.eth0.ipv6.addresses = [
            {
              address = localAddress;
              prefixLength = 64;
            }
          ];
          interfaces.lo = {
            ipv4.addresses = [
              {
                address = ipv4;
                prefixLength = 32;
              }
            ];
            ipv6.addresses = [
              {
                address = ipv6;
                prefixLength = 128;
              }
            ];
          };
          defaultGateway6 = hostAddress;

          firewall.allowedTCPPorts = [
            22
            80
            443
          ];
        };

        # The container has no IPv4 gateway of its own; hand v4 to the host
        # over the v6 link the same way the music container does.
        systemd.services.ipv4-via-ipv6-route = {
          after = [ "network.target" ];
          wantedBy = [ "multi-user.target" ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = "${pkgs.iproute2}/bin/ip route add 0.0.0.0/0 via inet6 ${hostAddress}";
          };
        };
      };
  };

  systemd.services.tunnl-container-route = {
    after = [ "container@tunnl.service" ];
    bindsTo = [ "container@tunnl.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = pkgs.writeShellScript "add-tunnl-routes" ''
        ${pkgs.iproute2}/bin/ip route del ${ipv4}/32 via inet6 ${localAddress} || true
        ${pkgs.iproute2}/bin/ip -6 route del ${ipv6}/128 via inet6 ${localAddress} || true
        ${pkgs.iproute2}/bin/ip route add ${ipv4}/32 via inet6 ${localAddress} || true
        ${pkgs.iproute2}/bin/ip -6 route add ${ipv6}/128 via inet6 ${localAddress} || true
      '';
      ExecStop = pkgs.writeShellScript "del-tunnl-routes" ''
        ${pkgs.iproute2}/bin/ip route del ${ipv4}/32 via inet6 ${localAddress} || true
        ${pkgs.iproute2}/bin/ip -6 route del ${ipv6}/128 via inet6 ${localAddress} || true
      '';
    };
  };
}
