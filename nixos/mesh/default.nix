{
  config,
  lib,
  pkgs,
  options,
  hive,
  ...
}:

let
  inherit (lib) mkOption types;
  cfg = config.mesh;
  ygg = config.services.yggdrasil;
  self = config.networking.hostName;

  hosts = import ./hosts.nix;
  others = lib.filterAttrs (name: _: name != self) hosts;
  hubs = lib.filterAttrs (_: h: h ? endpoint) others;
  keyed = lib.filterAttrs (_: h: h.publicKey != null) hosts;

  port = 9444;
  multicastPort = 9445;
  keyDir = "/var/lib/yggdrasil-key";
  password = "@MESH_PASSWORD@";
  credentials = "/run/credentials/yggdrasil.service";

  isHub = hosts.${self} ? endpoint;

  template = pkgs.writers.writeJSON "yggdrasil.conf" (
    lib.filterAttrs (_: v: v != null) ygg.settings
    // {
      PrivateKeyPath = "${credentials}/private-key";
    }
  );
in
{
  options.mesh = {
    enable = lib.mkEnableOption "the private yggdrasil mesh";

    multicastInterfaces = mkOption {
      type = types.nullOr types.str;
      default = "^(en|eth|wl).*";
      description = "Interfaces to discover other mesh nodes on, or null for none.";
    };
  };

  config = lib.mkMerge [
    (lib.mkIf cfg.enable {
      assertions = [
        {
          assertion = hosts ? ${self};
          message = "mesh: ${self} is not in nixos/mesh/hosts.nix";
        }
      ];

      sops.secrets.mesh-password = {
        sopsFile = ../../secrets/mesh.yaml;
        key = "password";
      };

      services.yggdrasil = {
        enable = true;
        group = "wheel";
        openMulticastPort = cfg.multicastInterfaces != null;
        denyDhcpcdInterfaces = [ "ygg0" ];
        settings = {
          PrivateKeyPath = "${keyDir}/private.pem";
          IfName = "ygg0";
          AdminListen = "unix:///run/yggdrasil/yggdrasil.sock";
          NodeInfoPrivacy = true;
          GroupPassword = password;
          Peers = lib.mapAttrsToList (_: h: "tls://${h.endpoint}:${toString port}?password=${password}") hubs;
          Listen = lib.optional isHub "tls://[::]:${toString port}?password=${password}";
          AllowedPublicKeys = lib.mapAttrsToList (_: h: h.publicKey) (
            lib.filterAttrs (name: _: name != self) keyed
          );
          MulticastInterfaces = lib.optional (cfg.multicastInterfaces != null) {
            Regex = cfg.multicastInterfaces;
            Beacon = true;
            Listen = true;
            Port = multicastPort;
            Password = password;
          };
        };
      };

      systemd.services.yggdrasil-key = {
        description = "Generate this node's yggdrasil key";
        wantedBy = [ "yggdrasil.service" ];
        before = [ "yggdrasil.service" ];
        unitConfig.ConditionPathExists = "!${keyDir}/private.pem";
        serviceConfig.Type = "oneshot";
        path = [ ygg.package ];
        script = ''
          umask 077
          mkdir -p ${keyDir}
          yggdrasil -genconf | yggdrasil -useconf -exportkey > ${keyDir}/private.pem.tmp
          mv ${keyDir}/private.pem.tmp ${keyDir}/private.pem
        '';
      };

      # The stock module writes settings into the store, so the shared
      # password is substituted in at start from a credential instead.
      systemd.services.yggdrasil = {
        requires = [ "yggdrasil-key.service" ];
        after = [ "yggdrasil-key.service" ];
        serviceConfig.LoadCredential = [ "mesh-password:${config.sops.secrets.mesh-password.path}" ];
        script = lib.mkForce ''
          install -m 600 ${template} "$RUNTIME_DIRECTORY/yggdrasil.conf"
          ${lib.getExe pkgs.replace-secret} '${password}' "$CREDENTIALS_DIRECTORY/mesh-password" "$RUNTIME_DIRECTORY/yggdrasil.conf"
          exec ${lib.getExe' ygg.package "yggdrasil"} -useconffile "$RUNTIME_DIRECTORY/yggdrasil.conf"
        '';
      };

      environment.systemPackages = [
        (pkgs.writeShellScriptBin "yggctl" ''
          exec ${lib.getExe' ygg.package "yggdrasilctl"} -endpoint=unix:///run/yggdrasil/yggdrasil.sock "$@"
        '')
      ];

      networking.firewall.allowedTCPPorts =
        lib.optional isHub port ++ lib.optional (cfg.multicastInterfaces != null) multicastPort;
    })

    (lib.optionalAttrs (options ? persist) (lib.mkIf cfg.enable { persist.directories = [ keyDir ]; }))

    (lib.optionalAttrs (options ? dns) {
      dns.records = lib.mapAttrs' (
        name: h:
        lib.nameValuePair "${name}.hive.hayl.in" { AAAA = hive.tools.yggdrasil.address h.publicKey; }
      ) keyed;
    })
  ];
}
