{
  config,
  lib,
  pkgs,
  ...
}:

let
  usersConfig = import ./users-config.nix;

  # Build user config with host SSH keys for fromHost users
  containerUsers = lib.mapAttrs (
    name: userCfg:
    userCfg
    // {
      # Inherit SSH keys from host for fromHost users
      sshKeys =
        if userCfg.fromHost or false then
          config.users.users.${name}.openssh.authorizedKeys.keys or [ ]
        else
          userCfg.sshKeys or [ ];
    }
  ) usersConfig.users;
in
{
  # Create service accounts on host with same UIDs as container users
  # Skip users with fromHost = true
  users.users = lib.mapAttrs (name: userCfg: {
    isSystemUser = true;
    uid = userCfg.uid;
    group = "media";
    home = "/var/empty";
    createHome = false;
    shell = pkgs.shadow; # /sbin/nologin
  }) (lib.filterAttrs (name: userCfg: !(userCfg.fromHost or false)) usersConfig.users);

  # Ensure the music directories exist with proper permissions
  systemd.tmpfiles.rules =
    [
      "d /bulk/music 0755 root media -"
      "d /persistent/containers/music-upload/ssh 0755 root root -"
    ]
    ++ lib.mapAttrsToList (
      name: userCfg: "d /bulk/music/${name} 0755 ${name} media -"
    ) usersConfig.users;

  # Container for music uploads
  containers.music-upload = {
    ephemeral = true; # Container is stateless except for bind mounts
    autoStart = true;

    # Use private network for container connectivity
    privateNetwork = true;
    hostAddress6 = "fd00::1";
    localAddress6 = "fd00::2";

    # Bind mount each user's music directory to their home
    bindMounts =
      lib.mapAttrs' (
        name: userCfg:
        lib.nameValuePair "/home/${name}/uploads" {
          hostPath = "/bulk/music/${name}";
          isReadOnly = false;
        }
      ) containerUsers
      // {
        # Persist SSH host keys
        "/etc/ssh" = {
          hostPath = "/persistent/containers/music-upload/ssh";
          isReadOnly = false;
        };
      };

    config =
      { config, pkgs, ... }:
      {
        # Basic system config
        system.stateVersion = "25.11";

        # Install useful tools for file transfers
        environment.systemPackages = with pkgs; [
          magic-wormhole-rs
          magic-wormhole
          rclone
          curl
          wget
          rsync
          unzip
          p7zip
          ffmpeg
          yt-dlp
          tmux
          screen
          dust
        ];

        # Create media group with GID from shared config
        users.groups = lib.mapAttrs (name: groupCfg: {
          gid = groupCfg.gid;
        }) usersConfig.groups;

        # Create users from shared config
        users.users = lib.mapAttrs (name: userCfg: {
          isNormalUser = true;
          uid = userCfg.uid;
          home = "/home/${name}";
          group = "media";
          extraGroups = [ "users" ] ++ (if userCfg.fromHost or false then [ "wheel" ] else [ ]);
          openssh.authorizedKeys.keys = userCfg.sshKeys or [ ];
        }) containerUsers;

        # Enable SSH
        services.openssh = {
          enable = true;
          settings = {
            PermitRootLogin = "no";
            PasswordAuthentication = false;
          };
        };

        # Set a nice MOTD
        users.motd = ''
          Welcome to the music upload container!

          Your home directory is your personal music folder.
          Available tools: magic-wormhole, curl, wget, yt-dlp, ffmpeg

          Example: wormhole receive <code>
        '';

        networking = {
          interfaces.eth0 = {
            ipv6.addresses = [
              {
                address = "fd00::2";
                prefixLength = 64;
              }
            ];
          };
          interfaces.lo = {
            ipv4.addresses = [
              {
                address = "23.144.156.5";
                prefixLength = 32;
              }
            ];
            ipv6.addresses = [
              {
                address = "2602:fbf5:3::5";
                prefixLength = 128;
              }
            ];
          };
          defaultGateway6 = "fd00::1";
        };

        # Add IPv4 default route via IPv6 gateway
        systemd.services.ipv4-via-ipv6-route = {
          after = [ "network.target" ];
          wantedBy = [ "multi-user.target" ];
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = "${pkgs.iproute2}/bin/ip route add 0.0.0.0/0 via inet6 fd00::1";
          };
        };
      };
  };

  systemd.services.music-container-route = {
    after = [ "container@music-upload.service" ];
    bindsTo = [ "container@music-upload.service" ]; # Restart when container restarts
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = pkgs.writeShellScript "add-music-routes" ''
        ${pkgs.iproute2}/bin/ip route del 23.144.156.5/32 via inet6 fd00::2 || true
        ${pkgs.iproute2}/bin/ip -6 route del 2602:fbf5:3::5/128 via inet6 fd00::2 || true
        ${pkgs.iproute2}/bin/ip route add 23.144.156.5/32 via inet6 fd00::2 || true
        ${pkgs.iproute2}/bin/ip -6 route add 2602:fbf5:3::5/128 via inet6 fd00::2 || true
      '';
      ExecStop = pkgs.writeShellScript "del-music-routes" ''
        ${pkgs.iproute2}/bin/ip route del 23.144.156.5/32 via inet6 fd00::2 || true
        ${pkgs.iproute2}/bin/ip -6 route del 2602:fbf5:3::5/128 via inet6 fd00::2 || true
      '';
    };
  };
}
