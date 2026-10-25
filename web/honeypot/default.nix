{
  pkgs,
  hive,
  lib,
  ...
}:

let
  naersk = pkgs.callPackage hive.sources.naersk.outPath { };
in
naersk.buildPackage {
  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type: !(type == "directory" && baseNameOf (toString path) == "target");
  };
}
