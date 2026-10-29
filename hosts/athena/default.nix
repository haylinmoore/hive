{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
{
  deployment.targetHost = "athena.infra.hayl.in";

  imports = [
    ./www.nix
    ./hardware-configuration.nix
    ./networking.nix
    ./power.nix
    ./impermanence.nix
    ./bgp.nix
    ./music

    ../../service/module.nix

    ./lambda.nix
    ./88x31.nix
    ./256.nix
    ./dollpublish.nix
    ./authentik.nix
    ./jellyfin.nix
    ./aconite.nix
    ./archive.nix
    ./rutorrent.nix
    ./fun.nix
    ./minecraft.nix
    ./skyfactory.nix
    ./thelounge.nix
    ./dial.nix
    ./certs.nix
    ./tunnl.nix
    ./knot.nix

    ../../nixos/certs/hayl-in.nix
    ../../nixos/certs/host-infra.nix
    ../../nixos/certs/estrogen-coffee.nix
    ../../nixos/certs/aconite-systems.nix
    ../../nixos/certs/uwu-estate.nix
  ];

  defaultHttp.enable = true;

  dns = {
    addresses = {
      v4 = "23.144.156.0";
      v6 = "2602:fbf5::";
    };
    ptr = {
      ${config.dns.addresses.v4} = "athena.infra.hayl.in";
      ${config.dns.addresses.v6} = "athena.infra.hayl.in";
    };
    vhosts.enable = true;
    records = {
      "athena.infra.hayl.in" = config.dns.here;
      "v4.athena.infra.hayl.in".A = config.dns.addresses.v4;
      "v6.athena.infra.hayl.in".AAAA = config.dns.addresses.v6;
      "404.hayl.in" = config.dns.here;
    };
  };

  users.mutableUsers = false;
  users.users.root.hashedPasswordFile = "/persistent/root.password";
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJcNOU+KapauS9OoI1aWNkeHohIp9DNg6fAqwJwtA0hR root@media"
  ];

  security.tpm2.enable = false;

  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [
    80
    443
    2222
  ];

  networking.firewall.allowedUDPPorts = [
    80
    443
  ];

  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = false;
    };
  };

  environment.systemPackages = with pkgs; [
    ethtool
    ipmitool
    mtr
    bird3
  ];

  security.pam.loginLimits = [
    {
      domain = "*";
      type = "soft";
      item = "nofile";
      value = "65536";
    }
    {
      domain = "*";
      type = "hard";
      item = "nofile";
      value = "65536";
    }
  ];

  services.hived.enable = true;
  services.hived.jobs = [
    "dns"
    "arin"
  ];

  mesh.enable = true;
  mesh.multicastInterfaces = null;

  sops.secrets."dns" = {
    sopsFile = ../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [
      "acme-hayl.in.service"
      "acme-athena.infra.hayl.in.service"
      "acme-uwu.estate.service"
      "acme-aconite.systems.service"
      "acme-estrogen.coffee.service"
      "acme-tunnl.hayl.in.service"
    ];
  };

  system.stateVersion = "25.11";
}
