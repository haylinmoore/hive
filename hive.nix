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
      ./nixos/modules # Pull in all nixos module
      ./nixos/shared/users.nix
      ./nixos/shared/default.nix
      ./nixos/shared/nix.nix
      "${sops}/modules/sops"
    ];

    deployment.buildOnTarget = true;
  };

  zoe = import ./hosts/zoe;

  athena = import ./hosts/athena;
}
