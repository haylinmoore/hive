{ hive, ... }:
{
  nix = {
    settings = {
      trusted-users = [
        "root"
        "haylin"
      ];
      experimental-features = "nix-command flakes";

      substituters = [
        "https://cache.nixos.org/"
      ];
      trusted-public-keys = [
        "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
      ];
    };

    registry.nixpkgs.to = {
      type = "path";
      path = hive.sources.nixpkgs;
    };

    nixPath = [ "nixpkgs=flake:nixpkgs" ];
  };
}
