{ pkgs, hive, ... }:

(import "${hive.nixpkgs.path}/nixos") {
  system = "x86_64-linux";
  configuration = {
    imports = [
      ./nixos.nix
      "${hive.sources.home-manager}/nixos"
      "${hive.sources.sops}/modules/sops"
    ];

    # Use the pkgs with overlays from hive
    nixpkgs.pkgs = hive.nixpkgs;

    # Pass hive to the module system
    _module.args = { inherit hive; };
  };
}
