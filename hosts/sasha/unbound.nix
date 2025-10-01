# configuration.nix
{ pkgs, lib, ... }:

let
  # Define common insecure DNS servers
  insecureDNSServers = [
    #"10.33.0.1"
    "9.9.9.10"
    "1.1.1.1"
  ];

  # Import captive portal domains
  captivePortalsByCategory = import ./captive-portals.nix;

  # Flatten all captive portal domains into a single list
  captivePortalDomains = lib.flatten (
    lib.mapAttrsToList (
      _category: categorySet:
      if builtins.isList categorySet then categorySet else lib.flatten (lib.attrValues categorySet)
    ) captivePortalsByCategory
  );

  # Define TLD configurations
  altTLDs = {
    alfis = {
      tlds = [
        "ygg"
        "anon"
        "btn"
        "conf"
        "index"
        "merch"
        "mirror"
        "mob"
        "screen"
        "srv"
      ];
      nameservers = [
        "324:71e:281a:9ed3::53"
        "300:6223::53"
      ];
    };

    furnic = {
      tlds = [ "fur" ];
      nameservers = [
        "138.197.140.189"
        "168.235.111.72"
        "162.243.19.47"
        "38.103.195.4"
      ];
    };

    opennic = {
      tlds = [
        "bbs"
        "chan"
        "cyb"
        "dyn"
        "epic"
        "geek"
        "gopher"
        "indy"
        "libre"
        "neo"
        "null"
        "o"
        "oss"
        "oz"
        "parody"
        "pirate"
        "free"
        "opennic.glue"
      ];
      nameservers = [
        "138.197.140.189"
        "168.235.111.72"
        "162.243.19.47"
        "38.103.195.4"
      ];
    };

    devhack = {
      tlds = [
        "core.devhack.net"
        "int.devhack.net"
      ];
      nameservers = [
        "10.213.8.1"
      ];
    };
  };

  # Generate all TLDs for domain-insecure
  allTlds = lib.flatten (lib.mapAttrsToList (_: cfg: cfg.tlds) altTLDs);

  # Generate forward-zone configurations
  forwardZones =
    lib.flatten (
      lib.mapAttrsToList (
        name: cfg:
        map (tld: {
          name = "${tld}.";
          forward-addr = cfg.nameservers;
          forward-tls-upstream = false;
        }) cfg.tlds
      ) altTLDs
    )
    ++ (map (domain: {
      name = "${domain}.";
      forward-addr = insecureDNSServers;
      forward-tls-upstream = false;
    }) captivePortalDomains);

  secureServerConfig = {
    logfile = "/tmp/unbound.log";
    verbosity = 3;
    interface = [ "127.0.0.1" ];
    port = 53;
    access-control = [ "127.0.0.1 allow" ];
    harden-glue = true;
    harden-dnssec-stripped = true;
    use-caps-for-id = false;
    prefetch = true;
    hide-identity = true;
    hide-version = true;
    domain-insecure = allTlds ++ captivePortalDomains;
  };

  insecureServerConfig = secureServerConfig // {
    harden-glue = false;
    harden-dnssec-stripped = false;
    hide-identity = false;
    hide-version = false;
    prefetch = false;
  };

  # Define secure and insecure upstream DNS configurations
  secureUpstreamDNS = {
    name = ".";
    forward-addr = [
      "9.9.9.9#dns.quad9.net"
      "149.112.112.112#dns.quad9.net"
      "2620:fe::fe#dns.quad9.net"
      "2620:fe::9#dns.quad9.net"
    ];
    forward-tls-upstream = true;
  };

  insecureUpstreamDNS = {
    name = ".";
    forward-addr = insecureDNSServers;
    forward-tls-upstream = false;
  };

  secureMode = false;

in
{
  services.unbound = {
    enable = true;
    settings = {
      remote-control = {
        control-enable = true;
        server-key-file = "/etc/unbound/unbound_server.key";
        server-cert-file = "/etc/unbound/unbound_server.pem";
        control-key-file = "/etc/unbound/unbound_control.key";
        control-cert-file = "/etc/unbound/unbound_control.pem";
      };
      server = if secureMode then secureServerConfig else insecureServerConfig;
      forward-zone = forwardZones ++ [
        (if secureMode then secureUpstreamDNS else insecureUpstreamDNS)
      ];
    };
  };
}
