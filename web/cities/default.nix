{
  pkgs,
  lib,
  stdenv,
  callPackage,
  ...
}:

let
  jsonAll = callPackage ./json_all.nix { };
  json1k = callPackage ./json_1k.nix { };
in
stdenv.mkDerivation {
  pname = "cities";
  version = "1.0";

  src = ./.;

  buildPhase = ''
    runHook preBuild

    mkdir -p $out

    # Copy JSON files from the individual derivations
    cp ${jsonAll}/us_cities_all.json $out/
    cp ${json1k}/us_cities_1k.json $out/

    # Copy index.html if it exists
    if [ -f "$src/index.html" ]; then
      cp "$src/index.html" $out/
    fi

    runHook postBuild
  '';

  installPhase = ''
    # Nothing to do, everything is already in $out
  '';

  passthru = {
    json_all = jsonAll;
    json_1k = json1k;
  };

  meta = {
    description = "Cities website with JSON data";
    platforms = lib.platforms.all;
  };
}
