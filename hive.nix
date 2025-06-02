let
  sources = import ./npins;
  pkgs = import sources.nixpkgs {};
in
{
  meta = {
    nixpkgs = pkgs;
  };

  defaults = { pkgs, ... }: {
    imports = [
      ./modules/nixos # Pull in all nixos module
      ./modules/shared/users.nix
    ];

    deployment.buildOnTarget = true;

    environment.systemPackages = with pkgs; [
      vim wget curl tree
    ];

    networking.nameservers = [ "9.9.9.10" "149.112.112.10" "2620:fe::10" "2620:fe::fe:10"];
    networking.dhcpcd.enable = false;
    
    security.acme.acceptTerms = true;
    security.acme.defaults.email = "acme@haylinmoore.com";

    services.openssh.enable = true;
    networking.firewall.enable = true;
  };

  bella = {
    deployment.targetHost = "bella.infra.hayl.in";

    networking.firewall.allowedTCPPorts = [ 80 443 2222 ];

    imports = [
      ./modules/hosts/bella/hardware-configuration.nix
      ./modules/hosts/bella/networking.nix
      ./modules/hosts/bella/services.nix
    ];
  };
}
