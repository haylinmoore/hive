let
  hive = import ./default.nix;
  sops = hive.sources.sops.outPath;
in
{
  meta = {
    nixpkgs = hive.pkgs;
    specialArgs = { inherit hive; };
  };

  defaults = {
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
