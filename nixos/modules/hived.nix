{
  config,
  lib,
  options,
  pkgs,
  hive,
  ...
}:

let
  cfg = config.services.hived;
  host = config.networking.hostName;

  hosts = import ../mesh/hosts.nix;
  address = name: hive.tools.yggdrasil.address hosts.${name}.publicKey;
  isHub = hosts.${host} ? endpoint;
  hubs = lib.attrNames (lib.filterAttrs (_: h: h ? endpoint) hosts);
  operators = lib.attrNames (lib.filterAttrs (_: h: h.operator or false) hosts);

  environment = {
    HIVED_HOST = host;
    HIVED_HUB = if isHub then "1" else "0";
    HIVED_AUTO_DEPLOY = if cfg.autoDeploy then "1" else "0";
    HIVED_AGENT_PORT = toString cfg.port;
    HIVED_AGENT_BIND = "[${address host}]:${toString cfg.port}";
    HIVED_PUBLIC_BIND = "127.0.0.1:${toString cfg.publicPort}";
    HIVED_DASH_BIND = "[::1]:${toString cfg.dashPort}";
    HIVED_NODES = lib.concatStringsSep "," (
      lib.mapAttrsToList (name: h: "${name}=${address name}=${h.publicKey}") hosts
    );
    HIVED_HUBS = lib.concatMapStringsSep "," address hubs;
    HIVED_OPERATORS = lib.concatMapStringsSep "," address operators;
    HIVED_STATE_DIR = cfg.stateDir;
    HIVED_REPO_URL = cfg.repoUrl;
    HIVED_BRANCH = cfg.branch;
    HIVED_IGNORED_UNITS = lib.concatStringsSep "," cfg.check.ignoredUnits;
    HIVED_SETTLE_SECS = toString cfg.check.settleSeconds;
    HIVED_QUEUED_MAX_AGE_SECS = toString cfg.queuedMaxAgeSeconds;
    HIVED_TIMEOUT_BUILD_SECS = toString cfg.timeouts.build;
    HIVED_TIMEOUT_ACTIVATE_SECS = toString cfg.timeouts.activate;
    HIVED_JOBS = lib.concatStringsSep "," cfg.jobs;
  };

  runnerPath = with pkgs; [
    git
    colmena
    nix
    systemd
    openssh
    bash
    coreutils
    gnutar
    gzip
    xz
  ];
in
{
  options.services.hived = {
    enable = lib.mkEnableOption "the hive deploy agent";

    jobs = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "dns"
          "arin"
        ]
      );
      default = [ ];
      description = "Pushes this node previews on every commit and runs once approved.";
    };

    autoDeploy = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Deploy new commits on arrival, rather than waiting for an approval.";
    };

    domain = lib.mkOption {
      type = lib.types.str;
      default = "hived.${host}.infra.hayl.in";
      description = "Public name GitHub pokes on a hub.";
    };

    useACMEHost = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = "${host}.infra.hayl.in";
      description = "Certificate to serve the poke endpoint under.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 15680;
      description = "Agent port on this node's mesh address.";
    };

    publicPort = lib.mkOption {
      type = lib.types.port;
      default = 15681;
      description = "Loopback port for the poke endpoint, behind nginx.";
    };

    dashPort = lib.mkOption {
      type = lib.types.port;
      default = 15682;
      description = "Loopback port for the dashboard, behind nginx.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/hived";
      description = "Where records, logs and the checkout live.";
    };

    repoUrl = lib.mkOption {
      type = lib.types.str;
      default = "https://github.com/haylinmoore/hive.git";
      description = "Pinned clone source. Never taken from a request.";
    };

    branch = lib.mkOption {
      type = lib.types.str;
      default = "main";
      description = "Only commits reachable from this branch may deploy.";
    };

    check = {
      settleSeconds = lib.mkOption {
        type = lib.types.int;
        default = 30;
        description = ''
          How long to watch for units failing after activation. A service that
          crashes five seconds in is still starting when switch returns.
        '';
      };
      ignoredUnits = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        description = "Units that never count as a new failure.";
      };
    };

    queuedMaxAgeSeconds = lib.mkOption {
      type = lib.types.int;
      default = 1800;
      description = "A request waiting longer than this is retired as stale.";
    };

    timeouts = {
      build = lib.mkOption {
        type = lib.types.int;
        default = 2700;
      };
      activate = lib.mkOption {
        type = lib.types.int;
        default = 600;
      };
    };
  };

  config = lib.mkIf cfg.enable (
    lib.mkMerge [
      {
        assertions = [
          {
            assertion = config.mesh.enable;
            message = "hived talks over the mesh, enable mesh on ${host}";
          }
        ];

        users.users.hived = {
          isSystemUser = true;
          group = "hived";
          home = cfg.stateDir;
        };
        users.groups.hived = { };

        systemd.tmpfiles.rules = [
          "d ${cfg.stateDir} 0755 hived hived - -"
          "d ${cfg.stateDir}/logs 0755 hived hived - -"
        ];

        systemd.services.hived = {
          description = "hive deploy agent";
          wantedBy = [ "multi-user.target" ];
          after = [
            "network-online.target"
            "yggdrasil.service"
          ];
          wants = [ "network-online.target" ];
          partOf = [ "yggdrasil.service" ];
          inherit environment;
          path = [
            pkgs.systemd
            pkgs.git
          ];

          serviceConfig = {
            ExecStart = "${hive.hived}/bin/hived serve";
            User = "hived";
            Group = "hived";
            SupplementaryGroups = [ config.services.yggdrasil.group ];
            Restart = "always";
            RestartSec = 2;

            NoNewPrivileges = true;
            CapabilityBoundingSet = "";
            ProtectSystem = "strict";
            ProtectHome = true;
            PrivateTmp = true;
            PrivateDevices = true;
            ReadWritePaths = [ cfg.stateDir ];
            RestrictAddressFamilies = [
              "AF_INET"
              "AF_INET6"
              "AF_UNIX"
            ];
            RestrictNamespaces = true;
            RestrictRealtime = true;
            LockPersonality = true;
            MemoryDenyWriteExecute = true;
            SystemCallArchitectures = "native";
            SystemCallFilter = [ "@system-service" ];
          };
        };

        systemd.services."hived-run@" = {
          description = "hive deploy runner %i";
          inherit environment;
          path = runnerPath;

          # Keeps switch-to-configuration from stopping the deploy that is switching.
          restartIfChanged = false;
          stopIfChanged = false;
          unitConfig.X-StopOnReconfiguration = false;

          serviceConfig = {
            Type = "oneshot";
            ExecStart = "${hive.hived}/bin/hived run %i";
            TimeoutStartSec = cfg.timeouts.build + cfg.timeouts.activate + 600;
            WorkingDirectory = cfg.stateDir;
          };
        };

        # The host key is a recipient through ssh-to-age, so sops needs the same conversion.
        systemd.services."hived-job@" = lib.mkIf (cfg.jobs != [ ]) {
          description = "hive job %i";
          environment = environment // {
            SOPS_AGE_KEY_CMD = "${lib.getExe pkgs.ssh-to-age} -private-key -i /etc/ssh/ssh_host_ed25519_key";
            NIX_PATH = "nixpkgs=${pkgs.path}";
          };
          path = runnerPath;
          restartIfChanged = false;
          stopIfChanged = false;
          unitConfig.X-StopOnReconfiguration = false;

          serviceConfig = {
            Type = "oneshot";
            ExecStart = "${hive.hived}/bin/hived job %i";
            TimeoutStartSec = cfg.timeouts.build + 600;
            WorkingDirectory = cfg.stateDir;
          };
        };

        # Off by default on headless hosts, which silently drops the rule below.
        security.polkit.enable = true;

        security.polkit.extraConfig = ''
          // hived may start its own runner and jobs and nothing else. The
          // instance is digits only, so there is no room to smuggle in another unit.
          polkit.addRule(function(action, subject) {
            if (subject.user !== "hived") return polkit.Result.NOT_HANDLED;
            if (action.id !== "org.freedesktop.systemd1.manage-units") return polkit.Result.NOT_HANDLED;
            var unit = action.lookup("unit");
            if (/^hived-(run|job)@[0-9]+\.service$/.test(unit)) {
              var verb = action.lookup("verb");
              if (verb === "start" || verb === "status") return polkit.Result.YES;
            }
            return polkit.Result.NOT_HANDLED;
          });
        '';

        networking.firewall.interfaces.ygg0.allowedTCPPorts = [
          cfg.port
        ]
        ++ lib.optionals isHub [
          80
          443
        ];
      }

      (lib.optionalAttrs (options ? persist) {
        persist.directories = [
          {
            directory = cfg.stateDir;
            user = "hived";
            group = "hived";
            mode = "0755";
          }
        ];
      })

      (lib.mkIf isHub {
        security.acme.certs."dash.hive.hayl.in" = {
          dnsProvider = "bunny";
          dnsPropagationCheck = true;
          environmentFile = "/run/secrets/dns";
        };

        services.nginx.virtualHosts."dash.hive.hayl.in" = {
          forceSSL = true;
          useACMEHost = "dash.hive.hayl.in";
          extraConfig = ''
            allow 200::/7;
            deny all;
          '';
          locations."/" = {
            proxyPass = "http://[::1]:${toString cfg.dashPort}";
            recommendedProxySettings = true;
          };
        };
      })

      (lib.optionalAttrs (options ? proxySites) (
        lib.mkIf isHub {
          proxySites.hived = {
            domain = cfg.domain;
            proxyUri = "http://127.0.0.1:${toString cfg.publicPort}/";
            useACMEHost = cfg.useACMEHost;
          };
        }
      ))
    ]
  );
}
