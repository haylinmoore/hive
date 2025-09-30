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
        ./nix/nixos # Pull in all nixos module
        ./nix/shared/users.nix
        ./nix/shared/default.nix
        "${sops}/modules/sops"
      ];

      deployment.buildOnTarget = true;
    };

  zoe = import ./nix/hosts/zoe;

  athena = import ./nix/hosts/athena;
}
