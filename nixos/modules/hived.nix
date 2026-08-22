{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

let
  cfg = config.services.hived;
  host = config.networking.hostName;

  environment = {
    HIVED_HOST = host;
    HIVED_BIND = "127.0.0.1:${toString cfg.port}";
    HIVED_STATE_DIR = cfg.stateDir;
    # The console and the GitHub Pages dashboard are the same file.
    HIVED_DASHBOARD = "${hive.web.hived}/index.html";
    HIVED_REPO_URL = cfg.repoUrl;
    HIVED_BRANCH = cfg.branch;
    HIVED_AUDIENCE = cfg.audience;
    HIVED_ALLOWED_REPOSITORY = cfg.allowedClaims.repository;
    HIVED_ALLOWED_REF = cfg.allowedClaims.reference;
    HIVED_IGNORED_UNITS = lib.concatStringsSep "," cfg.check.ignoredUnits;
    HIVED_SETTLE_SECS = toString cfg.check.settleSeconds;
    HIVED_QUEUED_MAX_AGE_SECS = toString cfg.queuedMaxAgeSeconds;
    HIVED_TIMEOUT_BUILD_SECS = toString cfg.timeouts.build;
    HIVED_TIMEOUT_ACTIVATE_SECS = toString cfg.timeouts.activate;
  }
  // lib.optionalAttrs (cfg.allowedClaims.ownerId != null) {
    HIVED_ALLOWED_OWNER_ID = cfg.allowedClaims.ownerId;
  }
  // lib.optionalAttrs (cfg.allowedClaims.workflowRef != null) {
    HIVED_ALLOWED_WORKFLOW_REF = cfg.allowedClaims.workflowRef;
  };

  # The runner needs everything a deploy touches, and nothing else does.
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
    enable = lib.mkEnableOption "the hive deploy daemon";

    domain = lib.mkOption {
      type = lib.types.str;
      default = "hived.${host}.infra.hayl.in";
      description = "Public name the GitHub Action posts to.";
    };

    useACMEHost = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = "${host}.infra.hayl.in";
      description = "Certificate to serve the daemon under.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 15680;
      description = "Loopback port the listener binds.";
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

    audience = lib.mkOption {
      type = lib.types.str;
      default = "hived";
      description = "Audience the workflow must request its OIDC token for.";
    };

    allowedClaims = {
      repository = lib.mkOption {
        type = lib.types.str;
        default = "haylinmoore/hive";
      };
      ownerId = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "8162992";
        description = "Numeric owner id, which survives a username being reused.";
      };
      reference = lib.mkOption {
        type = lib.types.str;
        default = "refs/heads/main";
      };
      workflowRef = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = "haylinmoore/hive/.github/workflows/deploy.yml@refs/heads/main";
        description = "Pin to one workflow, so a token from another will not do.";
      };
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

  config = lib.mkIf cfg.enable {
    users.users.hived = {
      isSystemUser = true;
      group = "hived";
      home = cfg.stateDir;
    };
    users.groups.hived = { };

    # Declared next to the thing that generates it. A no-op on hosts without
    # impermanence, which is why this is unconditional.
    persist.directories = [
      {
        directory = cfg.stateDir;
        user = "hived";
        group = "hived";
        mode = "0755";
      }
    ];

    systemd.tmpfiles.rules = [
      "d ${cfg.stateDir} 0755 hived hived - -"
      "d ${cfg.stateDir}/logs 0755 hived hived - -"
    ];

    # The unprivileged half. It can verify a token, write a record, render the
    # page, and ask systemd to start one specific unit. Nothing else.
    systemd.services.hived = {
      description = "hive deploy listener";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      inherit environment;
      path = [ pkgs.systemd ];

      serviceConfig = {
        ExecStart = "${hive.hived}/bin/hived serve";
        User = "hived";
        Group = "hived";
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

    # The privileged half. A fixed ExecStart with a numeric instance, so the
    # polkit rule below grants the listener exactly one capability.
    systemd.services."hived-run@" = {
      description = "hive deploy runner %i";
      inherit environment;
      path = runnerPath;

      # switch-to-configuration restarts the listener mid-deploy by design.
      # These keep it from stopping the deploy that is doing the restarting.
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

    # Headless hosts do not enable polkit by default, and without it the rule
    # below is silently dropped and the listener cannot start its runner.
    security.polkit.enable = true;

    security.polkit.extraConfig = ''
      // hived may start its own runner and nothing else. The instance is
      // digits only, so there is no room to smuggle in another unit.
      polkit.addRule(function(action, subject) {
        if (subject.user !== "hived") return polkit.Result.NOT_HANDLED;
        if (action.id !== "org.freedesktop.systemd1.manage-units") return polkit.Result.NOT_HANDLED;
        var unit = action.lookup("unit");
        if (/^hived-run@[0-9]+\.service$/.test(unit)) {
          var verb = action.lookup("verb");
          if (verb === "start" || verb === "status") return polkit.Result.YES;
        }
        return polkit.Result.NOT_HANDLED;
      });
    '';

    proxySites.hived = {
      domain = cfg.domain;
      proxyUri = "http://127.0.0.1:${toString cfg.port}/";
      useACMEHost = cfg.useACMEHost;
    };
  };
}
