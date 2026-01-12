{
  config,
  lib,
  pkgs,
  ...
}:

{
  environment.persistence."/persistent".directories = [
    "/var/lib/rutorrent"
    "/bulk/rutorrent"
    "/bulk/rtorrent"
  ];

  # Set permissions so media group can write
  systemd.tmpfiles.rules = [
    "d /bulk/rutorrent 0775 rutorrent media - -"
    "d /bulk/rtorrent 0775 rtorrent media - -"
    "d /bulk/downloads 0775 rtorrent media - -"
  ];

  services.rtorrent = {
    enable = true;
    dataDir = "/bulk/rtorrent";
    downloadDir = "/bulk/downloads";
    user = "rtorrent";
    group = "media";
    port = 50000;
    openFirewall = true;

    # Better settings for public torrents
    configText = ''
      # Enable DHT for better peer discovery
      dht.mode.set = auto
      dht.port.set = 6881

      # Enable PEX (peer exchange)
      protocol.pex.set = yes

      # Enable UDP tracker support
      trackers.use_udp.set = yes

      # Increase peer limits for better speeds
      throttle.max_peers.normal.set = 200
      throttle.max_peers.seed.set = 100

      # No download/upload rate limits
      throttle.global_down.max_rate.set_kb = 0
      throttle.global_up.max_rate.set_kb = 0
    '';
  };

  services.rutorrent = {
    enable = true;
    hostName = "rutorrent.local";
    dataDir = "/bulk/rutorrent";
    user = "rutorrent";
    group = "media";

    # Enable nginx for serving the PHP application
    nginx.enable = true;

    # Enable useful plugins
    plugins = [
      "httprpc"
      "edit" # Allows editing torrent properties including download path
      "data" # Shows torrent data files
      "datadir" # Change directory of existing torrents (move/relocate)
      "diskspace" # Shows disk space usage
      "erasedata" # Allows deleting torrents with data
      "trafic" # Traffic graphs
      "theme" # Theme support
      "autotools" # Automatic operations (e.g., move on completion based on label)
      "_getdir" # Required for autotools directory operations
      "create" # Create torrents from files
    ];
  };

  # Override the nginx virtualHost to listen only on localhost:8090
  services.nginx.virtualHosts."rutorrent.local" = {
    listen = [
      {
        addr = "127.0.0.1";
        port = 8090;
      }
    ];
  };
}
