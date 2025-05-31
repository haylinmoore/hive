let
  sources = import ./npins;
in
{
  meta = {
    nixpkgs = import sources.nixpkgs {};
  };

  defaults = { pkgs, ... }: {
    imports = [
      ./modules/shared/users.nix
    ];

    deployment.buildOnTarget = true;

    environment.systemPackages = with pkgs; [
      vim wget curl tree
    ];

    networking.nameservers = [ "9.9.9.10" "149.112.112.10" "2620:fe::10" "2620:fe::fe:10"];
    networking.dhcpcd.enable = false;

    services.openssh.enable = true;
    networking.firewall.enable = true;
  };

  bella = {
    deployment.targetHost = "bella.infra.hayl.in";

    imports = [
      ./modules/hosts/bella/hardware-configuration.nix
      ./modules/hosts/bella/networking.nix
    ];
  };
}
