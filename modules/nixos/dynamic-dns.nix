{
  config,
  lib,
  pkgs,
  ...
}:

let
  dli_pkg = pkgs.callPackage ../packages/dli.nix { };

  mkDynamicDNSService = name: cfg: {
    name = "dynamic-dns-${name}";
    value = {
      description = "Dynamic DNS updater for ${name}";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      serviceConfig = {
        Type = "oneshot";
        User = "nobody";
        Group = "nobody";
        EnvironmentFile = cfg.credentialsFile;
      };

      path = [
        pkgs.curl
        pkgs.gnugrep
        pkgs.gnused
        pkgs.iproute2
        pkgs.gawk
        dli_pkg
      ];

      script = ''
        set -euo pipefail

        # Get current IPv4 address from Cloudflare
        IPV4=$(${pkgs.curl}/bin/curl -s -4 https://cloudflare.com/cdn-cgi/trace | ${pkgs.gnugrep}/bin/grep '^ip=' | ${pkgs.gnused}/bin/sed 's/ip=//')

        # Get current IPv6 address
        ${lib.optionalString (cfg.ipv6Suffix != null) ''
          IPV6=$(${pkgs.iproute2}/bin/ip a s ${cfg.interface} | ${pkgs.gawk}/bin/awk '/::'${lib.escapeShellArg cfg.ipv6Suffix}'/ && !/inet6 f/ { split($2, a, "/"); print a[1] }')
        ''}
        ${lib.optionalString (cfg.ipv6Suffix == null) ''
          IPV6=$(${pkgs.curl}/bin/curl -s -6 https://cloudflare.com/cdn-cgi/trace | ${pkgs.gnugrep}/bin/grep '^ip=' | ${pkgs.gnused}/bin/sed 's/ip=//' || echo "")
        ''}

        echo "Current IPv4: $IPV4"
        if [ -n "$IPV6" ]; then
          echo "Current IPv6: $IPV6" 
        fi

        # Update A record
        if [ -n "$IPV4" ]; then
          echo "Updating A record for ${cfg.record} to $IPV4 with TTL ${toString cfg.ttl}"
          ${dli_pkg}/bin/dli --provider ${cfg.provider} --zone ${cfg.zone} set A ${cfg.record} "$IPV4" --ttl ${toString cfg.ttl}
        fi

        # Update AAAA record if IPv6 is available
        if [ -n "$IPV6" ]; then
          echo "Updating AAAA record for ${cfg.record} to $IPV6 with TTL ${toString cfg.ttl}"
          ${dli_pkg}/bin/dli --provider ${cfg.provider} --zone ${cfg.zone} set AAAA ${cfg.record} "$IPV6" --ttl ${toString cfg.ttl}
        fi
      '';
    };
  };

  mkDynamicDNSTimer = name: cfg: {
    name = "dynamic-dns-${name}";
    value = {
      description = "Timer for dynamic DNS updater ${name}";
      wantedBy = [ "timers.target" ];

      timerConfig = {
        OnCalendar = cfg.interval;
        Persistent = true;
        RandomizedDelaySec = "30min";
      };
    };
  };
in

{
  options.services.dynamic-dns = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          provider = lib.mkOption {
            type = lib.types.str;
            description = "DNS provider (e.g., bunny, cloudflare, duckdns, etc.)";
          };

          zone = lib.mkOption {
            type = lib.types.str;
            description = "DNS zone to manage";
          };

          record = lib.mkOption {
            type = lib.types.str;
            description = "DNS record name to update";
          };

          credentialsFile = lib.mkOption {
            type = lib.types.path;
            description = "Path to environment file containing provider credentials";
          };

          interval = lib.mkOption {
            type = lib.types.str;
            default = "*:0/15"; # Every 15 minutes
            description = "How often to check and update DNS records (systemd calendar format)";
          };

          ipv6Suffix = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "IPv6 suffix to look for in local interface addresses (e.g., '243'). If null, uses Cloudflare trace.";
          };

          interface = lib.mkOption {
            type = lib.types.str;
            default = "enp4s0";
            description = "Network interface to check for IPv6 address when using ipv6Suffix";
          };

          ttl = lib.mkOption {
            type = lib.types.int;
            default = 1;
            description = "TTL (Time To Live) in seconds for DNS records";
          };
        };
      }
    );
    default = { };
    description = "Dynamic DNS configurations";
  };

  config = lib.mkIf (config.services.dynamic-dns != { }) {
    systemd.services = lib.listToAttrs (
      lib.mapAttrsToList mkDynamicDNSService config.services.dynamic-dns
    );

    systemd.timers = lib.listToAttrs (lib.mapAttrsToList mkDynamicDNSTimer config.services.dynamic-dns);

    environment.systemPackages = [ dli_pkg ];
  };
}
