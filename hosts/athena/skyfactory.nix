{
  config,
  lib,
  pkgs,
  ...
}:
let
  dataDir = "/var/lib/skyfactory";

  serverProperties = pkgs.writeText "server.properties" ''
    generator-settings=2;0;1;
    op-permission-level=4
    allow-nether=true
    level-name=world
    enable-query=false
    allow-flight=true
    announce-player-achievements=true
    server-port=25566
    level-type=DEFAULT
    enable-rcon=false
    force-gamemode=false
    level-seed=
    server-ip=
    max-build-height=256
    spawn-npcs=true
    white-list=true
    spawn-animals=true
    snooper-enabled=false
    hardcore=false
    online-mode=true
    resource-pack=
    pvp=true
    difficulty=1
    enable-command-block=false
    gamemode=0
    player-idle-timeout=0
    max-players=10
    spawn-monsters=true
    generate-structures=true
    view-distance=10
    spawn-protection=0
    motd=mrow mrow
  '';

  whitelist = pkgs.writeText "whitelist.json" (
    builtins.toJSON [
      {
        uuid = "cb4fbf5b-a7fc-4eb8-9f08-f245afa62898";
        name = "haylin";
      }
      {
        uuid = "bcbd6cc3-bddb-4334-a3ed-18af305cc162";
        name = "1sreal";
      }
    ]
  );
in
{
  environment.persistence."/persistent".directories = [
    "/var/lib/skyfactory"
  ];

  networking.firewall.allowedTCPPorts = [ 25566 ];

  virtualisation.oci-containers.containers.skyfactory = {
    image = "daltonsbaker/skyfactory2_5";
    ports = [ "25566:25566" ];
    volumes = [ "${dataDir}:/data" ];
    cmd = [
      "java"
      "-Xms4096M"
      "-Xmx4096M"
      "-jar"
      "forge-1.7.10-10.13.4.1614-1.7.10-universal.jar"
      "nogui"
    ];
    workdir = "/data";
    extraOptions = [
      "--tty"
      "--interactive"
    ];
    autoStart = true;
  };

  systemd.services.podman-skyfactory.preStart = lib.mkAfter ''
    cp -f ${serverProperties} ${dataDir}/server.properties
    cp -f ${whitelist} ${dataDir}/whitelist.json
    echo "eula=true" > ${dataDir}/eula.txt
    chmod a+rw ${dataDir}/server.properties ${dataDir}/whitelist.json ${dataDir}/eula.txt
  '';
}
