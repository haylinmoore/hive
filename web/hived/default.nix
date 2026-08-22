{ pkgs, ... }:

pkgs.stdenv.mkDerivation rec {
  pname = "hived-dashboard";
  version = "1.0";

  src = ./.;

  buildPhase = ''
    mkdir -p $out
    cp -r ${src}/* $out/
  '';
}
