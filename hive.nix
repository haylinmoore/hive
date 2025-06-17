let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
  sops = sources.sops.outPath;
in
{
  meta = {
    nixpkgs = pkgs;
  };

  defaults =
    { pkgs, ... }:
    {
      imports = [
        ./modules/nixos # Pull in all nixos module
        ./modules/shared/users.nix
        ./modules/shared/default.nix
        "${sops}/modules/sops"
      ];

      deployment.buildOnTarget = true;
    };

  bella = {
    deployment.targetHost = "bella.infra.hayl.in";

    networking.firewall.allowedTCPPorts = [
      80
      443
      2222
    ];

    imports = [
      ./modules/shared/pve.nix
      ./modules/hosts/bella/hardware-configuration.nix
      ./modules/hosts/bella/networking.nix
      ./modules/hosts/bella/services.nix
    ];

    sops.secrets."dollpublish" = {
      sopsFile = ./secrets/bella/dollpublish.json;
      key = "";
      format = "json";
      owner = "dollpublish";
      path = "/home/dollpublish/users.json";
      restartUnits = [ "dollpublish.service" ];
    };
  };

  maya = {
    deployment.targetHost = "maya.infra.hayl.in";

    networking.firewall.allowedTCPPorts = [
      80
      443
    ];

    imports = [
      ./modules/shared/pve.nix
      ./modules/hosts/maya/hardware-configuration.nix
      ./modules/hosts/maya/networking.nix
    ];
  };

  zoe = {
    deployment.targetHost = "zoe.infra.hayl.in";

    imports = [
      ./modules/hosts/zoe/hardware-configuration.nix
      ./modules/hosts/zoe/networking.nix
      ./modules/hosts/zoe/services.nix
    ];

    sops.secrets."duckdns" = {
      sopsFile = ./secrets/zoe/tokens.yaml;
      key = "duckdns";
    };
  };
}
