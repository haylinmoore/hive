{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
{
  deployment.targetHost = "athena.infra.hayl.in";

  proxySites.ygg-haylin = {
    domain = "ygg.hayl.in";
    proxyUri = "http://localhost:15641/";
    useACMEHost = "hayl.in";
  };

  services.derivations = [
    (hive.web.www.service {
      domain = "hayl.in";
      port = 15641;
      bindAddr = "127.0.0.1";
    })
  ];

  proxySites.www = {
    domain = "hayl.in";
    proxyUri = "http://127.0.0.1:15641/";
    useACMEHost = "hayl.in";
  };

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./impermanence.nix
    ./bgp.nix
    ./music

    ../../service/module.nix

    ../../nixos/services/soft-serve.nix
    ../../nixos/services/lambda.nix
    ../../nixos/services/88x31.nix
    ../../nixos/services/256.nix
    ../../nixos/services/dollpublish.nix
    ../../nixos/services/umaring.nix

    ../../nixos/certs/hayl-in.nix
    ../../nixos/certs/estrogen-coffee.nix
  ];

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

  sops.secrets."dns" = {
    sopsFile = ../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [
      "acme-hayl.in.service"
    ];
  };

  system.stateVersion = "25.11";
}
