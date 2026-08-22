{
  pkgs,
  hive,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };

  package = naersk.buildPackage {
    src = ./.;
  };
in

package
