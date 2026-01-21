{ pkgs, hive, ... }:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };
in
naersk.buildPackage {
  src = hive.sources.umaring.outPath;
}
