let
  mono = import ./default.nix;
  sops = mono.sources.sops.outPath;
in
{
  meta = {
    nixpkgs = mono.pkgs;
  };

  defaults =
    { pkgs, ... }:
    {
      imports = [
        ./nix/mono-module.nix # Pull in mono as module arg
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
