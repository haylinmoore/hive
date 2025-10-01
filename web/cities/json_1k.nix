{
  pkgs,
  stdenv,
  fetchurl,
  ...
}:

stdenv.mkDerivation {
  pname = "us-cities-1k-json";
  version = "1.0";

  src = fetchurl {
    url = "https://gist.githubusercontent.com/Miserlou/c5cd8364bf9b2420bb29/raw/2bf258763cdddd704f8ffd3ea9a3e81d25e2c6f6/cities.json";
    sha256 = "sha256-UjPCgtxOq0fXK+QKQfnAlVLXt7+hWNRAa0fDYBXYXCE=";
  };

  nativeBuildInputs = with pkgs; [ jq ];

  dontUnpack = true;

  buildPhase = ''
    runHook preBuild

    jq '[.[] | {sc: .state, nm: .city, lat: (.latitude | tostring), lng: (.longitude | tostring)}] | group_by(.nm | ascii_downcase) | map({(.[0].nm | ascii_downcase): .}) | add' "$src" > us_cities_1k.json

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp us_cities_1k.json $out/

    runHook postInstall
  '';

  meta = {
    description = "Top 1k US cities JSON data in required format";
    platforms = pkgs.lib.platforms.all;
  };
}
