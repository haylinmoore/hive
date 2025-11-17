{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

rec {
  deployment.targetHost = "zoe.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./dnsmasq.nix
    ./home-assistant.nix
    ../../nixos/certs/uwu-estate.nix
    ../../nixos/modules/dli-ddns.nix
  ];

  networking.firewall.enable = false;

  # Add meshcore-cli for system-wide use
  environment.systemPackages = with pkgs; [
    hive.pkgs.python.meshcore-cli
  ];

  sops.secrets."dns" = {
    sopsFile = ../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [ "acme-uwu.estate.service" ];
  };

  services.dli-ddns.zoe-infra-hayl-in = {
    provider = "bunny";
    zone = "hayl.in";
    record = "zoe.infra";
    credentialsFile = "/run/secrets/dns";
    interval = "*:0/5";
    records = {
      A = {
        mode = "external";
      };
      AAAA = {
        mode = "interface";
        interface = "enp4s0";
        pattern = "^2.*243$";
      };
    };
  };

  services.headscale = {
    enable = true;
    port = 16483;
    settings = {
      dns.magic_dns = false;
      dns.nameservers.global = [
        "9.9.9.9"
        "149.112.112.112"
      ];
      server_url = "https://headscale.uwu.estate";
      logtail.enabled = false;
    };
  };
  services.tailscale.enable = true;

  proxySites.headscale = {
    domain = "headscale.uwu.estate";
    proxyUri = "http://localhost:${toString services.headscale.port}/";
    useACMEHost = "uwu.estate";
  };
}
