{ pkgs, ... }:

pkgs.stdenv.mkDerivation rec {
  pname = "256";
  version = "1.0";

  src = ./.;

  buildPhase = ''
    mkdir -p $out
    cp -r ${src}/* $out/
  '';
}
