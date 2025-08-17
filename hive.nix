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

  bella = import ./modules/hosts/bella;

  maya = import ./modules/hosts/maya;

  zoe = import ./modules/hosts/zoe;

  athena = import ./modules/hosts/athena;
}
