{
  config,
  lib,
  pkgs,
  ...
}:
let
  sources = import ../../npins;
in
{

  nix = {
    settings = {
      trusted-users = [
        "root"
        "haylin"
      ];
      experimental-features = "nix-command flakes";

      substituters = [
        "https://cache.nixos.org/"
      ];
      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      ];
    };

    registry.nixpkgs.to = {
      type = "path";
      path = sources.nixpkgs;
    };

    nixPath = [ "nixpkgs=flake:nixpkgs" ];

    gc = {
      automatic = true;
      options = "--delete-older-than 14d";
      randomizedDelaySec = "30min";
    };
  };

  sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];

  environment.systemPackages = with pkgs; [
    vim
    wget
    curl
    tree
    dig
    htop
  ];

  networking.nameservers = [
    "9.9.9.10"
    "149.112.112.10"
    "2620:fe::10"
    "2620:fe::fe:10"
  ];

  security.acme.acceptTerms = true;
  security.acme.defaults.email = "acme@haylinmoore.com";

  services.nginx.package = pkgs.nginxQuic;
  services.openssh.enable = true;
}
