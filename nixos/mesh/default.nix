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

  yggctl = pkgs.writeShellScriptBin "yggctl" ''
    exec ${lib.getExe' ygg.package "yggdrasilctl"} -endpoint=unix:///run/yggdrasil/yggdrasil.sock "$@"
  '';

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

      # Settings land in the store, so the password is substituted in at start.
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
        yggctl
        (pkgs.writeShellApplication {
          name = "ygg";
          excludeShellChecks = [ "SC2016" ];
          runtimeInputs = [
            yggctl
            pkgs.jq
            pkgs.util-linux
          ];
          text = ''
            names=${
              lib.escapeShellArg (
                builtins.toJSON (lib.mapAttrs' (name: h: lib.nameValuePair h.publicKey name) keyed)
              )
            }
            defs='
              def name: $names[.] // ("? " + .[0:16]);
              def dur: floor | if . < 60 then "\(.)s" elif . < 3600 then "\(. / 60 | floor)m" elif . < 86400 then "\(. / 3600 | floor)h" else "\(. / 86400 | floor)d" end;
              def size: if . < 1024 then "\(.)B" elif . < 1048576 then "\(. / 1024 | floor)K" else "\(. / 1048576 | floor)M" end;
            '

            case "''${1:-}" in
              peers)
                yggctl -json getPeers | jq -r --argjson names "$names" "$defs"'
                  ["PEER", "DIR", "VIA", "STATE", "UPTIME", "RTT", "RX", "TX", "COST", "ERROR"],
                  (.peers | sort_by(.key // "") | .[] | [
                    (.key // "" | name),
                    (if .inbound then "in" else "out" end),
                    (.remote | if test("%25") then "lan " + capture("%25(?<i>[^\\]]+)").i else capture("://(?<h>[^/]+)").h end),
                    (if .up then "up" else "down" end),
                    (.uptime // 0 | dur),
                    (if .latency then "\(.latency / 1e6 | floor)ms" else "-" end),
                    (.bytes_recvd // 0 | size),
                    (.bytes_sent // 0 | size),
                    (.cost // "-"),
                    (.last_error // "-")
                  ]) | @tsv' | column -t -s $'\t'
                ;;
              tree)
                yggctl -json getTree | jq -r --argjson names "$names" "$defs"'
                  ["NODE", "PARENT", "ADDRESS"],
                  (.tree | sort_by(.key | name) | .[] | [
                    (.key | name),
                    (if .parent == .key then "(root)" else .parent | name end),
                    .address
                  ]) | @tsv' | column -t -s $'\t'
                ;;
              *)
                exec yggctl "$@"
                ;;
            esac
          '';
        })
      ];

      networking.firewall.allowedTCPPorts =
        lib.optional isHub port ++ lib.optional (cfg.multicastInterfaces != null) multicastPort;
    })

    (lib.optionalAttrs (options ? persist) (lib.mkIf cfg.enable { persist.directories = [ keyDir ]; }))
  ];
}
