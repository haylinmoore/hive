let
  sources = import ./npins;
  pkgs = import sources.nixpkgs {
    config = {
      allowUnfree = true;
    };
  };
  lib = pkgs.lib;
  readTree = import ./tools/readTree.nix { };

  readHive =
    hiveArgs:
    readTree {
      path = ./.;
      args = hiveArgs;
      # Filter out directories we don't want readTree to process
      filter =
        parts: args:
        if
          (builtins.elem (builtins.head parts) [
            "secrets"
            "npins"
            ".git"
          ])
        then
          args // { __readTree = false; }
        else
          args;
    };
in
readTree.fix (
  self:
  (readHive {
    inherit lib;
    inherit sources;
    pkgs = pkgs;
    hive = self;
  })
  // {
    inherit sources;
    nixpkgs = pkgs;
  }
)
