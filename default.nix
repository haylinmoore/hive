let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
  lib = pkgs.lib;
  readTree = import ./nix/readTree.nix { };

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
            "nix"
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
    inherit pkgs lib;
    hive = self;
    inherit sources;
  })
  // {
    inherit sources pkgs;
  }
)
