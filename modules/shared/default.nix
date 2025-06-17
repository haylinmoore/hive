{ config, lib, pkgs, ... }:

{
  sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];

  environment.systemPackages = with pkgs; [
    vim
    wget
    curl
    tree
  ];

  networking.nameservers = [ "9.9.9.10" "149.112.112.10" "2620:fe::10" "2620:fe::fe:10" ];
  networking.dhcpcd.enable = false;

  security.acme.acceptTerms = true;
  security.acme.defaults.email = "acme@haylinmoore.com";

  services.openssh.enable = true;
  networking.firewall.enable = true;
}
