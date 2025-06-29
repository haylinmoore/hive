{
  config,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.services.duckdns-ds;
  duckdns-ds = pkgs.writeShellScriptBin "duckdns-ds" ''
    log_success=0
    log_error=0
    error_messages=""

    # Get primary IPv6 address for egress using Quad9's IPv6 DNS server
    PRIMARY_IPV6=$(ip a s enp4s0 | awk '/::${cfg.ipv6Suffix}/ && !/inet6 f/ { split($2, a, "/"); print a[1] }')

    # Try IPv4 update
    DRESPONSE_IPV4=$(curl -sS --max-time 60 --no-progress-meter -k -4 -K- <<< "url = \"https://www.duckdns.org/update?verbose=true&domains=$DUCKDNS_DOMAINS&token=$DUCKDNS_TOKEN&ip=\"")
    RESPONSE_IPV4=$(echo "$DRESPONSE_IPV4" | awk 'NR==1')
    IPV4=$(echo "$DRESPONSE_IPV4" | awk 'NR==2')
    IPCHANGE_IPV4=$(echo "$DRESPONSE_IPV4" | awk 'NR==4')

    if [[ "$RESPONSE_IPV4" = "OK" ]]; then
        if [[ "$IPCHANGE_IPV4" = "UPDATED" ]]; then
            echo "Your IPv4 was updated at $(date) to: $IPV4"
            log_success=1
        elif [[ "$IPCHANGE_IPV4" = "NOCHANGE" ]]; then
            echo "DuckDNS IPv4 request at $(date) successful. IP unchanged."
            log_success=1
        fi
    else
        error_messages+="IPv4 update failed: $DRESPONSE_IPV4\n"
        log_error=1
    fi

    # Try IPv6 update using the determined primary IPv6 address
    if [[ -n "$PRIMARY_IPV6" ]]; then
        DRESPONSE_IPV6=$(curl -sS --max-time 60 --no-progress-meter -k -K- <<< "url = \"https://www.duckdns.org/update?verbose=true&domains=$DUCKDNS_DOMAINS&token=$DUCKDNS_TOKEN&ipv6=$PRIMARY_IPV6\"")
        RESPONSE_IPV6=$(echo "$DRESPONSE_IPV6" | awk 'NR==1')
        IPV6_DUCKDNS=$(echo "$DRESPONSE_IPV6" | awk 'NR==2')
        IPCHANGE_IPV6=$(echo "$DRESPONSE_IPV6" | awk 'NR==4')

        if [[ "$RESPONSE_IPV6" = "OK" ]]; then
            if [[ "$IPCHANGE_IPV6" = "UPDATED" ]]; then
                echo "Your IPv6 was updated at $(date) to: $IPV6_DUCKDNS (using local primary IP: $PRIMARY_IPV6)"
                log_success=1
            elif [[ "$IPCHANGE_IPV6" = "NOCHANGE" ]]; then
                echo "DuckDNS IPv6 request at $(date) successful. IP unchanged (using local primary IP: $PRIMARY_IPV6)."
                log_success=1
            fi
        else
            error_messages+="IPv6 update failed: $DRESPONSE_IPV6\n"
            log_error=1
        fi
    else
        error_messages+="Could not determine primary IPv6 address for egress.\n"
        log_error=1
    fi

    if [[ "$log_success" -eq 0 ]] && [[ "$log_error" -eq 1 ]]; then
        echo -e "Both IPv4 and IPv6 updates failed. Please check your settings.\n$error_messages"
        exit 1
    elif [[ "$log_success" -eq 0 ]]; then
        echo "No successful updates and no specific errors logged. Unexpected state."
        exit 1
    fi
  '';
in
{
  options.services.duckdns-ds = {
    enable = lib.mkEnableOption "DuckDNS Dynamic DNS Client";
    tokenFile = lib.mkOption {
      default = null;
      type = lib.types.path;
      description = ''
        The path to a file containing the token
        used to authenticate with DuckDNS.
      '';
    };

    domains = lib.mkOption {
      default = null;
      type = lib.types.nullOr (lib.types.listOf lib.types.str);
      example = [ "examplehost" ];
      description = ''
        The domain(s) to update in DuckDNS
        (without the .duckdns.org suffix)
      '';
    };

    domainsFile = lib.mkOption {
      default = null;
      type = lib.types.nullOr lib.types.path;
      example = lib.literalExpression ''
        pkgs.writeText "duckdns-domains.txt" '''
          examplehost
          examplehost2
          examplehost3
        '''
      '';
      description = ''
        The path to a file containing a
        newline-separated list of DuckDNS
        domain(s) to be updated
        (without the .duckdns.org suffix)
      '';
    };

    ipv6Suffix = lib.mkOption {
      default = "243"; # Default suffix
      type = lib.types.str;
      description = ''
        The suffix of the IPv6 address to be used for updates.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.domains != null || cfg.domainsFile != null;
        message = "Either services.duckdns-ds.domains or services.duckdns-ds.domainsFile has to be defined";
      }
      {
        assertion = !(cfg.domains != null && cfg.domainsFile != null);
        message = "services.duckdns-ds.domains and services.duckdns-ds.domainsFile can't both be defined at the same time";
      }
      {
        assertion = (cfg.tokenFile != null);
        message = "services.duckdns-ds.tokenFile has to be defined";
      }
    ];

    users.users.duckdns = {
      isSystemUser = true;
      group = "duckdns";
    };
    users.groups.duckdns = { };

    environment.systemPackages = [ duckdns-ds ];

    systemd.services.duckdns-ds = {
      description = "DuckDNS Dynamic DNS Client";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      startAt = "*:3/5";
      path = [
        pkgs.gnused
        pkgs.systemd
        pkgs.curl
        pkgs.gawk
        pkgs.iproute2
        duckdns-ds
      ];
      serviceConfig = {
        Type = "simple";
        User = "duckdns";
        Group = "duckdns";
        LoadCredential = [
          "DUCKDNS_TOKEN_FILE:${cfg.tokenFile}"
        ] ++ lib.optionals (cfg.domainsFile != null) [ "DUCKDNS_DOMAINS_FILE:${cfg.domainsFile}" ];
      };
      script = ''
        export DUCKDNS_TOKEN=$(systemd-creds cat DUCKDNS_TOKEN_FILE)
        ${lib.optionalString (cfg.domains != null) ''
          export DUCKDNS_DOMAINS='${lib.strings.concatStringsSep "," cfg.domains}'
        ''}
        ${lib.optionalString (cfg.domainsFile != null) ''
          export DUCKDNS_DOMAINS=$(systemd-creds cat DUCKDNS_DOMAINS_FILE | sed -z 's/\n/,/g')
        ''}
        exec ${lib.getExe duckdns-ds}
      '';
    };
  };

  # Forked `duckdns` by notthebee
  meta.maintainers = with lib.maintainers; [
    notthebee
    haylinmoore
  ];
}
