{ pkgs, ... }:

pkgs.stdenv.mkDerivation {
  pname = "us-cities-all-json";
  version = "1.0";

  src = pkgs.fetchurl {
    url = "https://raw.githubusercontent.com/lutangar/cities.json/0025d6fc08a699555a27f9f614fd6c24f394242b/cities.json";
    sha256 = "sha256-zj1hAha8zjTvyw18ELCiJvoWS8yU1KCY+S4+f1SCJLk=";
  };

  nativeBuildInputs = with pkgs; [ jq ];

  dontUnpack = true;

  buildPhase = ''
    runHook preBuild

    jq '[.[] | select(.country == "US") | {sc: .admin1, nm: (if .name == "East New York" then "New York" else .name end), lat: .lat, lng: .lng}] | group_by(.nm | ascii_downcase) | map({(.[0].nm | ascii_downcase): .}) | add' "$src" > us_cities_all.json

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp us_cities_all.json $out/

    runHook postInstall
  '';

  meta = {
    description = "All US cities JSON data in required format";
    platforms = pkgs.lib.platforms.all;
  };
}
