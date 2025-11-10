{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  # Convert service derivation to systemd unit
  toSystemdService = svc: {
    description = svc.name;

    wantedBy = [ "multi-user.target" ];
    wants = svc.wants;
    requires = svc.requires;
    after = svc.after;

    preStart = mkIf (svc.preStart != null) svc.preStart;
    postStart = mkIf (svc.postStart != null) svc.postStart;
    preStop = mkIf (svc.preStop != null) svc.preStop;
    postStop = mkIf (svc.postStop != null) svc.postStop;

    serviceConfig = mkMerge [
      (mkIf (svc.command != null) {
        ExecStart = svc.command;
        Restart = "always";
      })
      (mkIf (svc.environment != { }) {
        Environment = mapAttrsToList (n: v: "${n}=${v}") svc.environment;
      })
      (mkIf (svc.user != null) {
        User = svc.user;
      })
      (mkIf (svc.group != null) {
        Group = svc.group;
      })
      (mkIf (svc.workingDirectory != null) {
        WorkingDirectory = svc.workingDirectory;
      })
    ];
  };

in

{
  options.services.derivations = mkOption {
    type = types.listOf types.attrs;
    default = [ ];
    description = "List of service derivations";
  };

  config = {
    # Generate systemd services
    systemd.services = listToAttrs (
      map (svc: {
        name = "derivations-${svc.name}";
        value = toSystemdService svc;
      }) config.services.derivations
    );
  };
}
