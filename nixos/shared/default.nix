{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
{
  nix.gc = {
    automatic = true;
    options = "--delete-older-than 14d";
    randomizedDelaySec = "30min";
  };

  sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];

  environment.systemPackages = with pkgs; [
    hello
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

  services.openssh.enable = true;
}
