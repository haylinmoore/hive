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
      ./modules/nixos # Pull in all nixos module
      ./modules/shared/users.nix
      ./modules/shared/default.nix
      ./modules/shared/nix.nix
      "${sops}/modules/sops"
    ];

    deployment.buildOnTarget = true;
  };

  zoe = import ./modules/hosts/zoe;

  athena = import ./modules/hosts/athena;
}
