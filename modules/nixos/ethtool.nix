{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.networking.interfaces;

  ethtoolOpts =
    { name, ... }:
    {
      options = {
        ethtool = {
          speed = mkOption {
            type = types.nullOr types.int;
            default = null;
            description = "Link speed in Mbps (e.g., 25000 for 25Gb)";
          };

          duplex = mkOption {
            type = types.nullOr (
              types.enum [
                "half"
                "full"
              ]
            );
            default = null;
            description = "Duplex mode (half or full)";
          };

          autoneg = mkOption {
            type = types.nullOr types.bool;
            default = null;
            description = "Enable or disable auto-negotiation";
          };

          fec = mkOption {
            type = types.nullOr (
              types.enum [
                "auto"
                "off"
                "rs"
                "baser"
                "llrs"
              ]
            );
            default = null;
            description = "Forward Error Correction encoding";
          };
        };
      };
    };

  # Generate ethtool commands for an interface
  generateEthtoolCommands =
    name: opts:
    let
      speedDuplexCmd =
        if opts.ethtool.autoneg == false && opts.ethtool.speed != null && opts.ethtool.duplex != null then
          "${pkgs.ethtool}/bin/ethtool -s ${name} speed ${toString opts.ethtool.speed} duplex ${opts.ethtool.duplex} autoneg off"
        else if opts.ethtool.autoneg == true then
          "${pkgs.ethtool}/bin/ethtool -s ${name} autoneg on"
        else if opts.ethtool.autoneg == false then
          "${pkgs.ethtool}/bin/ethtool -s ${name} autoneg off"
        else
          "";

      fecCmd =
        if opts.ethtool.fec != null then
          "${pkgs.ethtool}/bin/ethtool --set-fec ${name} encoding ${opts.ethtool.fec}"
        else
          "";
    in
    lib.concatStringsSep "\n" (
      filter (x: x != "") [
        speedDuplexCmd
        fecCmd
      ]
    );

  # Get all interfaces with ethtool configuration
  interfacesWithEthtool = lib.filterAttrs (
    name: opts:
    opts.ethtool.speed != null
    || opts.ethtool.duplex != null
    || opts.ethtool.autoneg != null
    || opts.ethtool.fec != null
  ) cfg;

in
{
  options = {
    networking.interfaces = mkOption {
      type = types.attrsOf (types.submodule ethtoolOpts);
    };
  };

  config = mkIf (interfacesWithEthtool != { }) {
    systemd.services = mapAttrs' (
      name: opts:
      nameValuePair "network-interface-${name}-ethtool" {
        description = "Configure ethtool settings for ${name}";
        after = [ "sys-subsystem-net-devices-${name}.device" ];
        before = [ "network.target" ];
        wantedBy = [ "sys-subsystem-net-devices-${name}.device" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
        };
        script = generateEthtoolCommands name opts;
      }
    ) interfacesWithEthtool;
  };
}
