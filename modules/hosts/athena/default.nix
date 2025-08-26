{
  config,
  lib,
  pkgs,
  ...
}:

{
  deployment.targetHost = "athena.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./impermanence.nix
    ./bgp.nix
    ./music

    ../../services/www.nix
    ../../services/soft-serve.nix
    ../../services/lambda.nix
    ../../services/88x31.nix
    ../../services/256.nix
    ../../services/dollpublish.nix

    ../../certs/hayl-in.nix
    ../../certs/estrogen-coffee.nix
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
    sopsFile = ../../../secrets/dns.env;
    format = "dotenv";
    owner = "acme";
    restartUnits = [
      "acme-hayl.in.service"
    ];
  };

  system.stateVersion = "25.11";
}
