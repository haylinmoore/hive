{
  config,
  lib,
  options,
  ...
}:

let
  inherit (lib) mkOption types;
  cfg = config.dns;

  strs = types.coercedTo types.str lib.singleton (types.listOf types.str);

  record = types.submodule {
    options = {
      A = mkOption {
        type = strs;
        default = [ ];
      };
      AAAA = mkOption {
        type = strs;
        default = [ ];
      };
      CNAME = mkOption {
        type = types.nullOr types.str;
        default = null;
      };
      NS = mkOption {
        type = strs;
        default = [ ];
      };
      TXT = mkOption {
        type = strs;
        default = [ ];
      };
      fromVhost = mkOption {
        type = types.bool;
        default = false;
        internal = true;
      };
    };
  };

  vhostNames = lib.concatLists (
    lib.mapAttrsToList (
      name: vhost:
      [ (if vhost.serverName == null then name else vhost.serverName) ] ++ vhost.serverAliases
    ) config.services.nginx.virtualHosts
  );

  plain = name: name != "_" && !lib.hasInfix "*" name && !lib.hasPrefix "~" name;

  fromVhost = if cfg.vhosts.cname != null then { CNAME = cfg.vhosts.cname; } else cfg.here;
in
{
  options.dns = {
    addresses = {
      v4 = mkOption {
        type = types.nullOr types.str;
        default = null;
      };
      v6 = mkOption {
        type = types.nullOr types.str;
        default = null;
      };
    };

    here = mkOption {
      type = types.attrs;
      readOnly = true;
      default = {
        A = lib.optional (cfg.addresses.v4 != null) cfg.addresses.v4;
        AAAA = lib.optional (cfg.addresses.v6 != null) cfg.addresses.v6;
      };
      description = "This host's addresses, ready to assign to a record.";
    };

    vhosts = {
      enable = lib.mkEnableOption "a record for every nginx virtual host in a managed zone";
      cname = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Point vhost records at this name instead of `dns.addresses`.";
      };
    };

    records = mkOption {
      type = types.attrsOf record;
      default = { };
      description = "Records keyed by fully qualified name.";
    };

    ptr = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Reverse names keyed by address.";
    };

    ignore = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Names managed by something else, which dnscontrol leaves alone.";
    };
  };

  config = {
    dns.records = lib.mkIf cfg.vhosts.enable (
      lib.genAttrs (lib.filter plain vhostNames) (_: lib.mkDefault (fromVhost // { fromVhost = true; }))
    );

    dns.ignore = lib.optionals (options.services ? dli-ddns) (
      lib.mapAttrsToList (_: ddns: "${ddns.record}.${ddns.zone}") config.services.dli-ddns
    );
  };
}
