{
  pkgs,
  hive,
  lib,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };

  package = naersk.buildPackage {
    # Nix paths ignore .gitignore, so without this filter the local cargo
    # target/ directory lands in the source hash and every local build produces
    # a different derivation than the nodes do.
    src = lib.cleanSourceWith {
      src = ./.;
      filter =
        path: type:
        let
          base = baseNameOf (toString path);
        in
        !(type == "directory" && base == "target");
    };
  };
in

package
