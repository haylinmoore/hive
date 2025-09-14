{
  config,
  lib,
  pkgs,
  ...
}:

let
  dli_pkg = pkgs.callPackage ../packages/dli.nix { };

  # Generate IP address fetching logic based on mode
  genIPAddressScript =
    recordType: recordCfg:
    let
      varName = if recordType == "A" then "IPV4" else "IPV6";
      curlFlag = if recordType == "A" then "-4" else "-6";
    in
    if recordCfg.mode == "external" then
      ''
        ${varName}=$(curl -s ${curlFlag} https://cloudflare.com/cdn-cgi/trace | grep '^ip=' | sed 's/ip=//' || echo "")
      ''
    else if recordCfg.mode == "interface" then
      if recordType == "A" then
        ''
          echo "Looking for IPv4 addresses on interface ${recordCfg.interface}${
            lib.optionalString (recordCfg.pattern != null) " matching pattern: ${recordCfg.pattern}"
          }"
          ip a s ${recordCfg.interface} | awk '/inet / { print "Found IPv4:", $2 }'
          ${varName}=$(ip a s ${recordCfg.interface} | awk '
            /inet / && !/127\.0\.0\.1/ { 
              gsub(/\/.*/, "", $2); 
              ip = $2;
              ${lib.optionalString (recordCfg.pattern != null) ''
                if (ip ~ /${lib.escapeShellArg recordCfg.pattern}/) {
                  print ip;
                  exit;
                }
              ''}
              ${lib.optionalString (recordCfg.pattern == null) ''
                print ip;
                exit;
              ''}
            }
          ')
          if [ -z "$${varName}" ]; then
            echo "No IPv4 address found on interface ${recordCfg.interface}${
              lib.optionalString (recordCfg.pattern != null) " matching pattern: ${recordCfg.pattern}"
            }"
          fi
        ''
      else
        ''
          echo "Looking for IPv6 addresses on interface ${recordCfg.interface}${
            lib.optionalString (recordCfg.pattern != null) " matching pattern: ${recordCfg.pattern}"
          }"
          ip a s ${recordCfg.interface} | awk '/inet6/ { print "Found IPv6:", $2 }'
          ${varName}=$(ip a s ${recordCfg.interface} | awk '
            /inet6/ && !/::1/ { 
              gsub(/\/.*/, "", $2); 
              ip = $2;
              ${lib.optionalString (recordCfg.pattern != null) ''
                if (ip ~ /${lib.escapeShellArg recordCfg.pattern}/) {
                  print ip;
                  exit;
                }
              ''}
              ${lib.optionalString (recordCfg.pattern == null) ''
                if (ip !~ /^fe80:/) {
                  print ip;
                  exit;
                }
              ''}
            }
          ')
          if [ -z "$${varName}" ]; then
            echo "No IPv6 address found on interface ${recordCfg.interface}${
              lib.optionalString (recordCfg.pattern != null) " matching pattern: ${recordCfg.pattern}"
            }"
          fi
        ''
    else
      "";

  # Generate DNS update script for a record type
  genDNSUpdateScript =
    recordType: recordCfg: cfg:
    if recordType == "A" then
      ''
        if [ -n "$IPV4" ]; then
          echo "Updating ${recordType} record for ${cfg.record} to $IPV4 with TTL ${toString cfg.ttl}"
          dli --provider ${cfg.provider} --zone ${cfg.zone} set ${recordType} ${cfg.record} "$IPV4" --ttl ${toString cfg.ttl}
          dli --provider ${cfg.provider} --zone ${cfg.zone} set ${recordType} v4.${cfg.record} "$IPV4" --ttl ${toString cfg.ttl}
        fi
      ''
    else
      ''
        if [ -n "$IPV6" ]; then
          echo "Updating ${recordType} record for ${cfg.record} to $IPV6 with TTL ${toString cfg.ttl}"
          dli --provider ${cfg.provider} --zone ${cfg.zone} set ${recordType} ${cfg.record} "$IPV6" --ttl ${toString cfg.ttl}
          dli --provider ${cfg.provider} --zone ${cfg.zone} set ${recordType} v6.${cfg.record} "$IPV6" --ttl ${toString cfg.ttl}
        fi
      '';

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

        ${lib.optionalString (cfg.records ? A) (genIPAddressScript "A" cfg.records.A)}
        ${lib.optionalString (cfg.records ? AAAA) (genIPAddressScript "AAAA" cfg.records.AAAA)}

        ${lib.optionalString (cfg.records ? A) ''
          if [ -n "$IPV4" ]; then
            echo "Current IPv4: $IPV4"
          else
            echo "Current IPv4: (not found)"
          fi
        ''}
        ${lib.optionalString (cfg.records ? AAAA) ''
          if [ -n "$IPV6" ]; then
            echo "Current IPv6: $IPV6" 
          else
            echo "Current IPv6: (not found)"
          fi
        ''}

        ${lib.optionalString (cfg.records ? A) (genDNSUpdateScript "A" cfg.records.A cfg)}
        ${lib.optionalString (cfg.records ? AAAA) (genDNSUpdateScript "AAAA" cfg.records.AAAA cfg)}
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
  options.services.dli-ddns = lib.mkOption {
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

          ttl = lib.mkOption {
            type = lib.types.int;
            default = 1;
            description = "TTL (Time To Live) in seconds for DNS records";
          };

          records = lib.mkOption {
            type = lib.types.attrsOf (
              lib.types.submodule {
                options = {
                  mode = lib.mkOption {
                    type = lib.types.enum [
                      "external"
                      "interface"
                    ];
                    description = "Mode for obtaining IP address: 'external' uses Cloudflare trace, 'interface' uses local interface";
                  };

                  interface = lib.mkOption {
                    type = lib.types.nullOr lib.types.str;
                    default = null;
                    description = "Network interface to check for IP address when using 'interface' mode";
                  };

                  pattern = lib.mkOption {
                    type = lib.types.nullOr lib.types.str;
                    default = null;
                    description = "Pattern to match IP address when using 'interface' mode (useful for IPv6 filtering)";
                  };
                };
              }
            );
            default = { };
            description = "Configuration for each DNS record type (A, AAAA)";
          };
        };
      }
    );
    default = { };
    description = "DLI Dynamic DNS configurations";
  };

  config = lib.mkIf (config.services.dli-ddns != { }) {
    systemd.services = lib.listToAttrs (
      lib.mapAttrsToList mkDynamicDNSService config.services.dli-ddns
    );

    systemd.timers = lib.listToAttrs (lib.mapAttrsToList mkDynamicDNSTimer config.services.dli-ddns);

    environment.systemPackages = [ dli_pkg ];
  };
}
