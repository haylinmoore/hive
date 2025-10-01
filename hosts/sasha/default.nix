{ pkgs, hive, ... }:

(import "${hive.nixpkgs.path}/nixos") {
  system = "x86_64-linux";
  configuration = {
    imports = [
      ./nixos.nix
      "${hive.sources.home-manager}/nixos"
    ];

    # Pass hive to the module system
    _module.args = { inherit hive; };
  };
}
