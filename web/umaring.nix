{ pkgs, hive, ... }:

let
  naersk = pkgs.callPackage hive.sources.naersk { };
in
naersk.buildPackage {
  src = hive.sources.umaring.outPath;
}
